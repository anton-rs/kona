//! Contains the logic for the `AttributesQueue` stage.

use crate::{
    stages::batch_queue::BatchQueue,
    traits::{
        ChainProvider, DataAvailabilityProvider, LogLevel, ResettableStage, SafeBlockFetcher,
        TelemetryProvider,
    },
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
        &self,
        l2_parent: L2BlockInfo,
        epoch: BlockID,
    ) -> anyhow::Result<PayloadAttributes>;
}

/// [AttributesQueue] accepts batches from the [super::BatchQueue] stage
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
pub struct AttributesQueue<DAP, CP, BF, T, AB>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
    BF: SafeBlockFetcher + Debug,
    T: TelemetryProvider + Debug,
    AB: AttributesBuilder + Debug,
{
    /// The rollup config.
    cfg: RollupConfig,
    /// The previous stage of the derivation pipeline.
    prev: BatchQueue<DAP, CP, BF, T>,
    /// Telemetry provider.
    telemetry: T,
    /// Whether the current batch is the last in its span.
    is_last_in_span: bool,
    /// The current batch being processed.
    batch: Option<SingleBatch>,
    /// The attributes builder.
    builder: AB,
}

impl<DAP, CP, BF, T, AB> AttributesQueue<DAP, CP, BF, T, AB>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
    BF: SafeBlockFetcher + Debug,
    T: TelemetryProvider + Debug,
    AB: AttributesBuilder + Debug,
{
    /// Create a new [AttributesQueue] stage.
    pub fn new(
        cfg: RollupConfig,
        prev: BatchQueue<DAP, CP, BF, T>,
        telemetry: T,
        builder: AB,
    ) -> Self {
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
impl<DAP, CP, BF, T, AB> ResettableStage for AttributesQueue<DAP, CP, BF, T, AB>
where
    DAP: DataAvailabilityProvider + Send + Debug,
    CP: ChainProvider + Send + Debug,
    BF: SafeBlockFetcher + Send + Debug,
    T: TelemetryProvider + Send + Debug,
    AB: AttributesBuilder + Send + Debug,
{
    async fn reset(&mut self, _: BlockInfo, _: SystemConfig) -> StageResult<()> {
        self.telemetry.write(Bytes::from("resetting attributes queue"), LogLevel::Info);
        // TODO: metrice the reset
        self.batch = None;
        self.is_last_in_span = false;
        Err(StageError::Eof)
    }
}
