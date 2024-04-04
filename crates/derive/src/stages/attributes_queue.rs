//! Contains the logic for the `AttributesQueue` stage.

use crate::{
    traits::{LogLevel, OriginProvider, ResettableStage, TelemetryProvider},
    types::{
        AttributesWithParent, BlockID, BlockInfo, L2BlockInfo, PayloadAttributes, ResetError,
        RollupConfig, SingleBatch, StageError, StageResult, SystemConfig,
    },
};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use core::fmt::Debug;

pub trait AttributesBuilder {
    /// Prepare the payload attributes.
    fn prepare_payload_attributes(
        &mut self,
        l2_parent: L2BlockInfo,
        epoch: BlockID,
    ) -> anyhow::Result<PayloadAttributes>;
}

/// [AttributesProvider] is a trait abstraction that generalizes the [BatchQueue] stage.
#[async_trait]
pub trait AttributesProvider {
    /// Returns the next valid batch upon the given safe head.
    async fn next_batch(&mut self, parent: L2BlockInfo) -> StageResult<SingleBatch>;

    /// Returns whether the current batch is the last in its span.
    fn is_last_in_span(&self) -> bool;
}

/// [AttributesQueue] accepts batches from the [BatchQueue] stage
/// and transforms them into [PayloadAttributes]. The outputted payload
/// attributes cannot be buffered because each batch->attributes transformation
/// pulls in data about the current L2 safe head.
///
/// [AttributesQueue] also buffers batches that have been output because
/// multiple batches can be created at once.
///
/// This stage can be reset by clearing its batch buffer.
/// This stage does not need to retain any references to L1 blocks.
#[derive(Debug)]
pub struct AttributesQueue<P, T, AB>
where
    P: AttributesProvider + OriginProvider + Debug,
    T: TelemetryProvider + Debug,
    AB: AttributesBuilder + Debug,
{
    /// The rollup config.
    cfg: RollupConfig,
    /// The previous stage of the derivation pipeline.
    prev: P,
    /// Telemetry provider.
    telemetry: T,
    /// Whether the current batch is the last in its span.
    is_last_in_span: bool,
    /// The current batch being processed.
    batch: Option<SingleBatch>,
    /// The attributes builder.
    builder: AB,
}

impl<P, T, AB> AttributesQueue<P, T, AB>
where
    P: AttributesProvider + OriginProvider + Debug,
    T: TelemetryProvider + Debug,
    AB: AttributesBuilder + Debug,
{
    /// Create a new [AttributesQueue] stage.
    pub fn new(cfg: RollupConfig, prev: P, telemetry: T, builder: AB) -> Self {
        Self { cfg, prev, telemetry, is_last_in_span: false, batch: None, builder }
    }

    /// Returns the L1 origin [BlockInfo].
    pub fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }

    /// Loads a batch from the previous stage if needed.
    pub async fn load_batch(&mut self, parent: L2BlockInfo) -> StageResult<SingleBatch> {
        if self.batch.is_none() {
            let batch = self.prev.next_batch(parent).await?;
            self.batch = Some(batch);
            self.is_last_in_span = self.prev.is_last_in_span();
        }
        self.batch.as_ref().cloned().ok_or(StageError::Eof)
    }

    /// Returns the next payload attributes from the current batch.
    pub async fn next_attributes(
        &mut self,
        parent: L2BlockInfo,
    ) -> StageResult<AttributesWithParent> {
        // Load the batch
        let batch = self.load_batch(parent).await?;

        // Construct the payload attributes from the loaded batch
        let attributes = self.create_next_attributes(batch, parent).await?;
        let populated_attributes =
            AttributesWithParent { attributes, parent, is_last_in_span: self.is_last_in_span };

        // Clear out the local state once we will succeed
        self.batch = None;
        self.is_last_in_span = false;
        Ok(populated_attributes)
    }

    /// Creates the next attributes.
    /// Transforms a [SingleBatch] into [PayloadAttributes].
    /// This sets `NoTxPool` and appends the batched transactions to the attributes transaction
    /// list.
    pub async fn create_next_attributes(
        &mut self,
        batch: SingleBatch,
        parent: L2BlockInfo,
    ) -> StageResult<PayloadAttributes> {
        // Sanity check parent hash
        if batch.parent_hash != parent.block_info.hash {
            return Err(StageError::Reset(ResetError::BadParentHash(
                batch.parent_hash,
                parent.block_info.hash,
            )));
        }

        // Sanity check timestamp
        let actual = parent.block_info.timestamp + self.cfg.block_time;
        if actual != batch.timestamp {
            return Err(StageError::Reset(ResetError::BadTimestamp(batch.timestamp, actual)));
        }

        // Prepare the payload attributes
        let tx_count = batch.transactions.len();
        let mut attributes = self.builder.prepare_payload_attributes(parent, batch.epoch())?;
        attributes.no_tx_pool = true;
        attributes.transactions.extend(batch.transactions);

        self.telemetry.write(
            Bytes::from(alloc::format!(
                "generated attributes in payload queue: txs={}, timestamp={}",
                tx_count,
                batch.timestamp,
            )),
            LogLevel::Info,
        );

        Ok(attributes)
    }
}

#[async_trait]
impl<P, T, AB> ResettableStage for AttributesQueue<P, T, AB>
where
    P: AttributesProvider + OriginProvider + Send + Debug,
    T: TelemetryProvider + Send + Debug,
    AB: AttributesBuilder + Send + Debug,
{
    async fn reset(&mut self, _: BlockInfo, _: SystemConfig) -> StageResult<()> {
        self.telemetry.write(Bytes::from("resetting attributes queue"), LogLevel::Info);
        // TODO: metrice the reset using telemetry
        // telemetry can provide a method of logging and metricing
        self.batch = None;
        self.is_last_in_span = false;
        Err(StageError::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::{AttributesQueue, L2BlockInfo, RollupConfig, StageError};
    use crate::{
        stages::test_utils::{new_mock_batch_queue, MockAttributesBuilder},
        traits::test_utils::TestTelemetry,
    };
    use alloc::vec;

    #[tokio::test]
    async fn test_load_batch_eof() {
        let cfg = RollupConfig::default();
        let telemetry = TestTelemetry::new();
        let mock_batch_queue = new_mock_batch_queue(None, vec![]);
        let mock_attributes_builder = MockAttributesBuilder::default();
        let mut attributes_queue =
            AttributesQueue::new(cfg, mock_batch_queue, telemetry, mock_attributes_builder);
        let parent = L2BlockInfo::default();
        let result = attributes_queue.load_batch(parent).await.unwrap_err();
        assert_eq!(result, StageError::Eof);
    }
}
