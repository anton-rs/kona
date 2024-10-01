//! Contains the logic for the `AttributesQueue` stage.

use alloc::{boxed::Box, sync::Arc};
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::{OptimismAttributesWithParent, OptimismPayloadAttributes};
use tracing::info;

use crate::{
    batch::SingleBatch,
    errors::{PipelineError, PipelineResult, ResetError},
    traits::{
        AttributesQueueBuilder, AttributesQueuePrior, NextAttributes, OriginAdvancer,
        OriginProvider, ResettableStage,
    },
};

/// [AttributesQueue] accepts batches from the [BatchQueue] stage
/// and transforms them into [OptimismPayloadAttributes].
///
/// The outputted payload attributes cannot be buffered because each batch->attributes
/// transformation pulls in data about the current L2 safe head.
///
/// [AttributesQueue] also buffers batches that have been output because
/// multiple batches can be created at once.
///
/// This stage can be reset by clearing its batch buffer.
/// This stage does not need to retain any references to L1 blocks.
///
/// [BatchQueue]: crate::stages::BatchQueue
#[derive(Debug)]
pub struct AttributesQueue<P: AttributesQueuePrior, B: AttributesQueueBuilder> {
    /// The rollup config.
    cfg: Arc<RollupConfig>,
    /// The previous stage of the derivation pipeline.
    prev: P,
    /// Whether the current batch is the last in its span.
    is_last_in_span: bool,
    /// The current batch being processed.
    batch: Option<SingleBatch>,
    /// The attributes builder.
    builder: B,
}

impl<P, B> AttributesQueue<P, B>
where
    P: AttributesQueuePrior,
    B: AttributesQueueBuilder,
{
    /// Create a new [AttributesQueue] stage.
    pub fn new(cfg: Arc<RollupConfig>, prev: P, builder: B) -> Self {
        crate::set!(STAGE_RESETS, 0, &["attributes-queue"]);
        Self { cfg, prev, is_last_in_span: false, batch: None, builder }
    }

    /// Loads a [SingleBatch] from the [AttributesQueuePrior] if needed.
    pub async fn load_batch(&mut self, parent: L2BlockInfo) -> PipelineResult<SingleBatch> {
        if self.batch.is_none() {
            let batch = self.prev.next_batch(parent).await?;
            self.batch = Some(batch);
            self.is_last_in_span = self.prev.is_last_in_span();
        }
        self.batch.as_ref().cloned().ok_or(PipelineError::Eof.temp())
    }

    /// Returns the next [OptimismAttributesWithParent] from the current batch.
    pub async fn next_attributes(
        &mut self,
        parent: L2BlockInfo,
    ) -> PipelineResult<OptimismAttributesWithParent> {
        crate::timer!(START, STAGE_ADVANCE_RESPONSE_TIME, &["attributes_queue"], timer);
        let batch = match self.load_batch(parent).await {
            Ok(batch) => batch,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                return Err(e);
            }
        };

        // Construct the payload attributes from the loaded batch.
        let attributes = match self.create_next_attributes(batch, parent).await {
            Ok(attributes) => attributes,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                return Err(e);
            }
        };
        let populated_attributes = OptimismAttributesWithParent {
            attributes,
            parent,
            is_last_in_span: self.is_last_in_span,
        };

        // Clear out the local state once payload attributes are prepared.
        self.batch = None;
        self.is_last_in_span = false;
        Ok(populated_attributes)
    }

    /// Creates the next attributes, transforming a [SingleBatch] into [OptimismPayloadAttributes].
    /// This sets `no_tx_pool` and appends the batched txs to the attributes tx list.
    pub async fn create_next_attributes(
        &mut self,
        batch: SingleBatch,
        parent: L2BlockInfo,
    ) -> PipelineResult<OptimismPayloadAttributes> {
        // Sanity check parent hash
        if batch.parent_hash != parent.block_info.hash {
            return Err(ResetError::BadParentHash(batch.parent_hash, parent.block_info.hash).into());
        }

        // Sanity check timestamp
        let actual = parent.block_info.timestamp + self.cfg.block_time;
        if actual != batch.timestamp {
            return Err(ResetError::BadTimestamp(batch.timestamp, actual).into());
        }

        // Prepare the payload attributes
        let tx_count = batch.transactions.len();
        let mut attributes = self.builder.prepare_payload_attributes(parent, batch.epoch()).await?;
        attributes.no_tx_pool = Some(true);
        match attributes.transactions {
            Some(ref mut txs) => txs.extend(batch.transactions),
            None => {
                if !batch.transactions.is_empty() {
                    attributes.transactions = Some(batch.transactions);
                }
            }
        }

        info!(
            target: "attributes-queue",
            "generated attributes in payload queue: txs={}, timestamp={}",
            tx_count, batch.timestamp
        );

        Ok(attributes)
    }
}

#[async_trait]
impl<P, AB> OriginAdvancer for AttributesQueue<P, AB>
where
    P: AttributesQueuePrior + Send,
    AB: AttributesQueueBuilder + Send,
{
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        self.prev.advance_origin().await
    }
}

#[async_trait]
impl<P, AB> NextAttributes for AttributesQueue<P, AB>
where
    P: AttributesQueuePrior + Send,
    AB: AttributesQueueBuilder + Send,
{
    async fn next_attributes(
        &mut self,
        parent: L2BlockInfo,
    ) -> PipelineResult<OptimismAttributesWithParent> {
        self.next_attributes(parent).await
    }
}

impl<P, AB> OriginProvider for AttributesQueue<P, AB>
where
    P: AttributesQueuePrior + Send,
    AB: AttributesQueueBuilder + Send,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P, AB> ResettableStage for AttributesQueue<P, AB>
where
    P: AttributesQueuePrior + Send,
    AB: AttributesQueueBuilder + Send,
{
    async fn reset(
        &mut self,
        block_info: BlockInfo,
        system_config: &SystemConfig,
    ) -> PipelineResult<()> {
        self.prev.reset(block_info, system_config).await?;
        self.batch = None;
        self.is_last_in_span = false;
        crate::inc!(STAGE_RESETS, &["attributes-queue"]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        errors::{BuilderError, PipelineErrorKind},
        stages::test_utils::{
            new_attributes_provider, MockAttributesBuilder, MockAttributesProvider,
        },
    };
    use alloc::{sync::Arc, vec, vec::Vec};
    use alloy_primitives::{b256, Address, Bytes, B256};
    use alloy_rpc_types_engine::PayloadAttributes;

    fn default_optimism_payload_attributes() -> OptimismPayloadAttributes {
        OptimismPayloadAttributes {
            payload_attributes: PayloadAttributes {
                timestamp: 0,
                suggested_fee_recipient: Address::default(),
                prev_randao: B256::default(),
                withdrawals: None,
                parent_beacon_block_root: None,
            },
            no_tx_pool: Some(false),
            transactions: None,
            gas_limit: None,
        }
    }

    fn new_attributes_queue(
        cfg: Option<RollupConfig>,
        origin: Option<BlockInfo>,
        batches: Vec<PipelineResult<SingleBatch>>,
    ) -> AttributesQueue<MockAttributesProvider, MockAttributesBuilder> {
        let cfg = cfg.unwrap_or_default();
        let mock_batch_queue = new_attributes_provider(origin, batches);
        let mock_attributes_builder = MockAttributesBuilder::default();
        AttributesQueue::new(Arc::new(cfg), mock_batch_queue, mock_attributes_builder)
    }

    #[tokio::test]
    async fn test_load_batch_eof() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let parent = L2BlockInfo::default();
        let result = attributes_queue.load_batch(parent).await.unwrap_err();
        assert_eq!(result, PipelineError::Eof.temp());
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
            PipelineErrorKind::Reset(ResetError::BadParentHash(Default::default(), bad_hash))
        );
    }

    #[tokio::test]
    async fn test_create_next_attributes_bad_timestamp() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let parent = L2BlockInfo::default();
        let batch = SingleBatch { timestamp: 1, ..Default::default() };
        let result = attributes_queue.create_next_attributes(batch, parent).await.unwrap_err();
        assert_eq!(result, PipelineErrorKind::Reset(ResetError::BadTimestamp(1, 0)));
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
        assert_eq!(result, PipelineErrorKind::Reset(ResetError::BadTimestamp(1, 2)));
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
        assert_eq!(result, PipelineErrorKind::Reset(ResetError::BadTimestamp(1, 2)));
    }

    #[tokio::test]
    async fn test_create_next_attributes_preparation_fails() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let parent = L2BlockInfo::default();
        let batch = SingleBatch::default();
        let result = attributes_queue.create_next_attributes(batch, parent).await.unwrap_err();
        assert_eq!(
            result,
            PipelineError::AttributesBuilder(BuilderError::AttributesUnavailable).crit()
        );
    }

    #[tokio::test]
    async fn test_create_next_attributes_success() {
        let cfg = RollupConfig::default();
        let mock = new_attributes_provider(None, vec![]);
        let mut payload_attributes = default_optimism_payload_attributes();
        let mock_builder =
            MockAttributesBuilder { attributes: vec![Ok(payload_attributes.clone())] };
        let mut aq = AttributesQueue::new(Arc::new(cfg), mock, mock_builder);
        let parent = L2BlockInfo::default();
        let txs = vec![Bytes::default(), Bytes::default()];
        let batch = SingleBatch { transactions: txs.clone(), ..Default::default() };
        let attributes = aq.create_next_attributes(batch, parent).await.unwrap();
        // update the expected attributes
        payload_attributes.no_tx_pool = Some(true);
        match payload_attributes.transactions {
            Some(ref mut t) => t.extend(txs),
            None => payload_attributes.transactions = Some(txs),
        }
        assert_eq!(attributes, payload_attributes);
    }

    #[tokio::test]
    async fn test_next_attributes_load_batch_eof() {
        let mut attributes_queue = new_attributes_queue(None, None, vec![]);
        let parent = L2BlockInfo::default();
        let result = attributes_queue.next_attributes(parent).await.unwrap_err();
        assert_eq!(result, PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_next_attributes_load_batch_last_in_span() {
        let cfg = RollupConfig::default();
        let mock = new_attributes_provider(None, vec![Ok(Default::default())]);
        let mut pa = default_optimism_payload_attributes();
        let mock_builder = MockAttributesBuilder { attributes: vec![Ok(pa.clone())] };
        let mut aq = AttributesQueue::new(Arc::new(cfg), mock, mock_builder);
        // If we load the batch, we should get the last in span.
        // But it won't take it so it will be available in the next_attributes call.
        let _ = aq.load_batch(L2BlockInfo::default()).await.unwrap();
        assert!(aq.is_last_in_span);
        assert!(aq.batch.is_some());
        // This should successfully construct the next payload attributes.
        // It should also reset the last in span flag and clear the batch.
        let attributes = aq.next_attributes(L2BlockInfo::default()).await.unwrap();
        pa.no_tx_pool = Some(true);
        let populated_attributes = OptimismAttributesWithParent {
            attributes: pa,
            parent: L2BlockInfo::default(),
            is_last_in_span: true,
        };
        assert_eq!(attributes, populated_attributes);
        assert!(!aq.is_last_in_span);
        assert!(aq.batch.is_none());
    }
}
