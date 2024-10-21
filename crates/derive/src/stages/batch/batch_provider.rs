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
    };
    use op_alloy_genesis::RollupConfig;
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
}
