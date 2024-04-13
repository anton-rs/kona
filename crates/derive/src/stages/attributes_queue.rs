//! Contains the logic for the `AttributesQueue` stage.

use crate::{
    traits::{OriginProvider, ResettableStage},
    types::{
        AttributesWithParent, BlockInfo, L2BlockInfo, PayloadAttributes, ResetError, RollupConfig,
        SingleBatch, StageError, StageResult, SystemConfig,
    },
};
use alloc::boxed::Box;
use async_trait::async_trait;
use core::fmt::Debug;
use tracing::info;

mod deposits;
pub(crate) use deposits::derive_deposits;

mod builder;
pub use builder::{AttributesBuilder, StatefulAttributesBuilder};

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
pub struct AttributesQueue<P, AB>
where
    P: AttributesProvider + OriginProvider + Debug,
    AB: AttributesBuilder + Debug,
{
    /// The rollup config.
    cfg: RollupConfig,
    /// The previous stage of the derivation pipeline.
    prev: P,
    /// Whether the current batch is the last in its span.
    is_last_in_span: bool,
    /// The current batch being processed.
    batch: Option<SingleBatch>,
    /// The attributes builder.
    builder: AB,
}

impl<P, AB> AttributesQueue<P, AB>
where
    P: AttributesProvider + OriginProvider + Debug,
    AB: AttributesBuilder + Debug,
{
    /// Create a new [AttributesQueue] stage.
    pub fn new(cfg: RollupConfig, prev: P, builder: AB) -> Self {
        Self { cfg, prev, is_last_in_span: false, batch: None, builder }
    }

    /// Loads a [SingleBatch] from the [AttributesProvider] if needed.
    pub async fn load_batch(&mut self, parent: L2BlockInfo) -> StageResult<SingleBatch> {
        if self.batch.is_none() {
            let batch = self.prev.next_batch(parent).await?;
            self.batch = Some(batch);
            self.is_last_in_span = self.prev.is_last_in_span();
        }
        self.batch.as_ref().cloned().ok_or(StageError::Eof)
    }

    /// Returns the next [AttributesWithParent] from the current batch.
    pub async fn next_attributes(
        &mut self,
        parent: L2BlockInfo,
    ) -> StageResult<L2AttributesWithParent> {
        // Load the batch.
        let batch = self.load_batch(parent).await?;

        // Construct the payload attributes from the loaded batch.
        let attributes = self.create_next_attributes(batch, parent).await?;
        let populated_attributes =
            L2AttributesWithParent { attributes, parent, is_last_in_span: self.is_last_in_span };

        // Clear out the local state once payload attributes are prepared.
        self.batch = None;
        self.is_last_in_span = false;
        Ok(populated_attributes)
    }

    /// Creates the next attributes, transforming a [SingleBatch] into [PayloadAttributes].
    /// This sets `no_tx_pool` and appends the batched txs to the attributes tx list.
    pub async fn create_next_attributes(
        &mut self,
        batch: SingleBatch,
        parent: L2BlockInfo,
    ) -> StageResult<L2PayloadAttributes> {
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
        let mut attributes = self
            .builder
            .prepare_payload_attributes(parent, batch.epoch())
            .await
            .map_err(StageError::AttributesBuild)?;
        attributes.no_tx_pool = true;
        attributes.transactions.extend(batch.transactions);

        info!(
            "generated attributes in payload queue: txs={}, timestamp={}",
            tx_count, batch.timestamp
        );

        Ok(attributes)
    }
}

impl<P, AB> OriginProvider for AttributesQueue<P, AB>
where
    P: AttributesProvider + OriginProvider + Debug,
    AB: AttributesBuilder + Debug,
{
    fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P, AB> ResettableStage for AttributesQueue<P, AB>
where
    P: AttributesProvider + OriginProvider + Send + Debug,
    AB: AttributesBuilder + Send + Debug,
{
    async fn reset(&mut self, _: BlockInfo, _: &SystemConfig) -> StageResult<()> {
        info!("resetting attributes queue");
        // TODO: metrice the reset using telemetry
        // telemetry can provide a method of logging and metricing
        self.batch = None;
        self.is_last_in_span = false;
        Err(StageError::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttributesQueue, BlockInfo, L2AttributesWithParent, L2BlockInfo, L2PayloadAttributes,
        RollupConfig, SingleBatch, StageError, StageResult,
    };
    use crate::{
        stages::test_utils::{
            new_attributes_provider, MockAttributesBuilder, MockAttributesProvider,
        },
        types::{BuilderError, RawTransaction},
    };
    use alloc::{vec, vec::Vec};
    use alloy_primitives::b256;

    fn new_attributes_queue(
        cfg: Option<RollupConfig>,
        origin: Option<BlockInfo>,
        batches: Vec<StageResult<SingleBatch>>,
    ) -> AttributesQueue<MockAttributesProvider, MockAttributesBuilder> {
        let cfg = cfg.unwrap_or_default();
        let mock_batch_queue = new_attributes_provider(origin, batches);
        let mock_attributes_builder = MockAttributesBuilder::default();
        AttributesQueue::new(cfg, mock_batch_queue, mock_attributes_builder)
    }

    #[tokio::test]
    async fn test_load_batch_eof() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let parent = L2BlockInfo::default();
        let result = attributes_queue.load_batch(parent).await.unwrap_err();
        assert_eq!(result, StageError::Eof);
    }

    #[tokio::test]
    async fn test_load_batch_last_in_span() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![Ok(Default::default())]);
        let parent = L2BlockInfo::default();
        let result = attributes_queue.load_batch(parent).await.unwrap();
        assert_eq!(result, Default::default());
        assert!(attributes_queue.is_last_in_span);
    }

    #[tokio::test]
    async fn test_create_next_attributes_bad_parent_hash() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let bad_hash = b256!("6666666666666666666666666666666666666666666666666666666666666666");
        let parent = L2BlockInfo {
            block_info: BlockInfo { hash: bad_hash, ..Default::default() },
            ..Default::default()
        };
        let batch = SingleBatch::default();
        let result = attributes_queue.create_next_attributes(batch, parent).await.unwrap_err();
        assert_eq!(
            result,
            StageError::Reset(super::ResetError::BadParentHash(Default::default(), bad_hash))
        );
    }

    #[tokio::test]
    async fn test_create_next_attributes_bad_timestamp() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let parent = L2BlockInfo::default();
        let batch = SingleBatch { timestamp: 1, ..Default::default() };
        let result = attributes_queue.create_next_attributes(batch, parent).await.unwrap_err();
        assert_eq!(result, StageError::Reset(super::ResetError::BadTimestamp(1, 0)));
    }

    #[tokio::test]
    async fn test_create_next_attributes_bad_parent_timestamp() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let parent = L2BlockInfo {
            block_info: BlockInfo { timestamp: 2, ..Default::default() },
            ..Default::default()
        };
        let batch = SingleBatch { timestamp: 1, ..Default::default() };
        let result = attributes_queue.create_next_attributes(batch, parent).await.unwrap_err();
        assert_eq!(result, StageError::Reset(super::ResetError::BadTimestamp(1, 2)));
    }

    #[tokio::test]
    async fn test_create_next_attributes_bad_config_timestamp() {
        let cfg = RollupConfig { block_time: 1, ..Default::default() };
        let mut attributes_queue = new_attributes_queue(Some(cfg), None, vec![]);
        let parent = L2BlockInfo {
            block_info: BlockInfo { timestamp: 1, ..Default::default() },
            ..Default::default()
        };
        let batch = SingleBatch { timestamp: 1, ..Default::default() };
        let result = attributes_queue.create_next_attributes(batch, parent).await.unwrap_err();
        assert_eq!(result, StageError::Reset(super::ResetError::BadTimestamp(1, 2)));
    }

    #[tokio::test]
    async fn test_create_next_attributes_preparation_fails() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let parent = L2BlockInfo::default();
        let batch = SingleBatch::default();
        let result = attributes_queue.create_next_attributes(batch, parent).await.unwrap_err();
        assert_eq!(
            result,
            StageError::AttributesBuild(BuilderError::Custom(anyhow::anyhow!(
                "missing payload attribute"
            )))
        );
    }

    #[tokio::test]
    async fn test_create_next_attributes_success() {
        let cfg = RollupConfig::default();
        let mock = new_attributes_provider(None, vec![]);
        let mut payload_attributes = L2PayloadAttributes::default();
        let mock_builder =
            MockAttributesBuilder { attributes: vec![Ok(payload_attributes.clone())] };
        let mut aq = AttributesQueue::new(cfg, mock, mock_builder);
        let parent = L2BlockInfo::default();
        let txs = vec![RawTransaction::default(), RawTransaction::default()];
        let batch = SingleBatch { transactions: txs.clone(), ..Default::default() };
        let attributes = aq.create_next_attributes(batch, parent).await.unwrap();
        // update the expected attributes
        payload_attributes.no_tx_pool = true;
        payload_attributes.transactions.extend(txs);
        assert_eq!(attributes, payload_attributes);
    }

    #[tokio::test]
    async fn test_next_attributes_load_batch_eof() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let parent = L2BlockInfo::default();
        let result = attributes_queue.next_attributes(parent).await.unwrap_err();
        assert_eq!(result, StageError::Eof);
    }

    #[tokio::test]
    async fn test_next_attributes_load_batch_last_in_span() {
        let cfg = RollupConfig::default();
        let mock = new_attributes_provider(None, vec![Ok(Default::default())]);
        let mut pa = L2PayloadAttributes::default();
        let mock_builder = MockAttributesBuilder { attributes: vec![Ok(pa.clone())] };
        let mut aq = AttributesQueue::new(cfg, mock, mock_builder);
        // If we load the batch, we should get the last in span.
        // But it won't take it so it will be available in the next_attributes call.
        let _ = aq.load_batch(L2BlockInfo::default()).await.unwrap();
        assert!(aq.is_last_in_span);
        assert!(aq.batch.is_some());
        // This should successfully construct the next payload attributes.
        // It should also reset the last in span flag and clear the batch.
        let attributes = aq.next_attributes(L2BlockInfo::default()).await.unwrap();
        pa.no_tx_pool = true;
        let populated_attributes = L2AttributesWithParent {
            attributes: pa,
            parent: L2BlockInfo::default(),
            is_last_in_span: true,
        };
        assert_eq!(attributes, populated_attributes);
        assert!(!aq.is_last_in_span);
        assert!(aq.batch.is_none());
    }
}
