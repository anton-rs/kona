//! This module contains the [BatchProvider] stage.

use super::NextBatchProvider;
use crate::{
    batch::SingleBatch,
    stages::{multiplexed::multiplexed_stage, AttributesProvider, BatchQueue, BatchValidator},
    traits::L2ChainProvider,
};
use core::fmt::Debug;
use op_alloy_protocol::L2BlockInfo;

multiplexed_stage!(
    BatchProvider<NextBatchProvider, F: L2ChainProvider>,
    additional_fields: {
        /// The L2 chain fetcher.
        fetcher: F,
    },
    stages: {
        BatchValidator => is_holocene_active,
    },
    default_stage: BatchQueue<F>(fetcher)
);

#[async_trait]
impl<P, F> AttributesProvider for BatchProvider<P, F>
where
    P: NextBatchProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug + Send,
    F: L2ChainProvider + Clone + Send + Debug,
{
    fn is_last_in_span(&self) -> bool {
        let Some(stage) = self.active_stage_ref() else {
            return false;
        };

        match stage {
            ActiveStage::BatchQueue(stage) => stage.is_last_in_span(),
            ActiveStage::BatchValidator(stage) => stage.is_last_in_span(),
        }
    }

    async fn next_batch(&mut self, parent: L2BlockInfo) -> PipelineResult<SingleBatch> {
        match self.active_stage_mut()? {
            ActiveStage::BatchQueue(stage) => stage.next_batch(parent).await,
            ActiveStage::BatchValidator(stage) => stage.next_batch(parent).await,
        }
    }
}

#[cfg(test)]
mod test {
    use super::BatchProvider;
    use crate::{
        stages::batch::batch_provider::ActiveStage,
        test_utils::{TestL2ChainProvider, TestNextBatchProvider},
        traits::{OriginProvider, ResetSignal, SignalReceiver},
    };
    use op_alloy_genesis::RollupConfig;
    use op_alloy_protocol::BlockInfo;
    use std::sync::Arc;

    #[test]
    fn test_batch_provider_validator_active() {
        let provider = TestNextBatchProvider::new(vec![]);
        let l2_provider = TestL2ChainProvider::default();
        let cfg = Arc::new(RollupConfig { holocene_time: Some(0), ..Default::default() });
        let mut batch_provider = BatchProvider::new(cfg, provider, l2_provider);

        let active_stage = batch_provider.active_stage_mut().unwrap();
        assert!(matches!(active_stage, ActiveStage::BatchValidator(_)));
    }

    #[test]
    fn test_batch_provider_batch_queue_active() {
        let provider = TestNextBatchProvider::new(vec![]);
        let l2_provider = TestL2ChainProvider::default();
        let cfg = Arc::new(RollupConfig::default());
        let mut batch_provider = BatchProvider::new(cfg, provider, l2_provider);

        let active_stage = batch_provider.active_stage_mut().unwrap();
        assert!(matches!(active_stage, ActiveStage::BatchQueue(_)));
    }

    #[test]
    fn test_batch_provider_transition_stage() {
        let provider = TestNextBatchProvider::new(vec![]);
        let l2_provider = TestL2ChainProvider::default();
        let cfg = Arc::new(RollupConfig { holocene_time: Some(2), ..Default::default() });
        let mut batch_provider = BatchProvider::new(cfg, provider, l2_provider);

        let active_stage = batch_provider.active_stage_mut().unwrap();

        // Update the L1 origin to Holocene activation.
        let ActiveStage::BatchQueue(stage) = active_stage else {
            panic!("Expected BatchQueue");
        };
        stage.prev.origin = Some(BlockInfo { number: 1, timestamp: 2, ..Default::default() });

        // Transition to the BatchValidator stage.
        let active_stage = batch_provider.active_stage_mut().unwrap();
        assert!(matches!(active_stage, ActiveStage::BatchValidator(_)));

        assert_eq!(batch_provider.origin().unwrap().number, 1);
    }

    #[test]
    fn test_batch_provider_transition_stage_backwards() {
        let provider = TestNextBatchProvider::new(vec![]);
        let l2_provider = TestL2ChainProvider::default();
        let cfg = Arc::new(RollupConfig { holocene_time: Some(2), ..Default::default() });
        let mut batch_provider = BatchProvider::new(cfg, provider, l2_provider);

        let active_stage = batch_provider.active_stage_mut().unwrap();

        // Update the L1 origin to Holocene activation.
        let ActiveStage::BatchQueue(stage) = active_stage else {
            panic!("Expected BatchQueue");
        };
        stage.prev.origin = Some(BlockInfo { number: 1, timestamp: 2, ..Default::default() });

        // Transition to the BatchValidator stage.
        let active_stage = batch_provider.active_stage_mut().unwrap();
        let ActiveStage::BatchValidator(stage) = active_stage else {
            panic!("Expected ChannelBank");
        };

        // Update the L1 origin to before Holocene activation, to simulate a re-org.
        stage.prev.origin = Some(BlockInfo::default());

        let active_stage = batch_provider.active_stage_mut().unwrap();
        assert!(matches!(active_stage, ActiveStage::BatchQueue(_)));
    }

    #[tokio::test]
    async fn test_batch_provider_reset_bq() {
        let provider = TestNextBatchProvider::new(vec![]);
        let l2_provider = TestL2ChainProvider::default();
        let cfg = Arc::new(RollupConfig::default());
        let mut batch_provider = BatchProvider::new(cfg, provider, l2_provider);

        // Reset the batch provider.
        batch_provider.signal(ResetSignal::default().signal()).await.unwrap();

        let Ok(ActiveStage::BatchQueue(batch_queue)) = batch_provider.active_stage_mut() else {
            panic!("Expected ");
        };
        assert!(batch_queue.l1_blocks.len() == 1);
    }

    #[tokio::test]
    async fn test_batch_provider_reset_validator() {
        let provider = TestNextBatchProvider::new(vec![]);
        let l2_provider = TestL2ChainProvider::default();
        let cfg = Arc::new(RollupConfig { holocene_time: Some(0), ..Default::default() });
        let mut batch_provider = BatchProvider::new(cfg, provider, l2_provider);

        // Reset the batch provider.
        batch_provider.signal(ResetSignal::default().signal()).await.unwrap();

        let Ok(ActiveStage::BatchValidator(validator)) = batch_provider.active_stage_mut() else {
            panic!("Expected BatchValidator");
        };
        assert!(validator.l1_blocks.len() == 1);
    }
}
