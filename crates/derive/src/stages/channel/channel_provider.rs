//! This module contains the [ChannelProvider] stage.

use super::{ChannelAssembler, ChannelBank, ChannelReaderProvider, NextFrameProvider};
use crate::stages::multiplexed::multiplexed_stage;
use alloy_primitives::Bytes;
use core::fmt::Debug;

multiplexed_stage!(
    ChannelProvider<NextFrameProvider>,
    stages: {
        ChannelAssembler => is_holocene_active,
    }
    default_stage: ChannelBank
);

#[async_trait]
impl<P> ChannelReaderProvider for ChannelProvider<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn next_data(&mut self) -> PipelineResult<Option<Bytes>> {
        match self.active_stage_mut()? {
            ActiveStage::ChannelAssembler(stage) => stage.next_data().await,
            ActiveStage::ChannelBank(stage) => stage.next_data().await,
        }
    }
}

#[cfg(test)]
mod test {
    use super::{ActiveStage, ChannelProvider};
    use crate::{prelude::OriginProvider, test_utils::TestNextFrameProvider};
    use alloc::sync::Arc;
    use op_alloy_genesis::RollupConfig;
    use op_alloy_protocol::BlockInfo;

    #[test]
    fn test_channel_provider_assembler_active() {
        let provider = TestNextFrameProvider::new(vec![]);
        let cfg = Arc::new(RollupConfig { holocene_time: Some(0), ..Default::default() });
        let mut channel_provider = ChannelProvider::new(cfg, provider);

        let active_stage = channel_provider.active_stage_mut().unwrap();
        assert!(matches!(active_stage, ActiveStage::ChannelAssembler(_)));
    }

    #[test]
    fn test_channel_provider_bank_active() {
        let provider = TestNextFrameProvider::new(vec![]);
        let cfg = Arc::new(RollupConfig::default());
        let mut channel_provider = ChannelProvider::new(cfg, provider);

        let active_stage = channel_provider.active_stage_mut().unwrap();
        assert!(matches!(active_stage, ActiveStage::ChannelBank(_)));
    }

    #[test]
    fn test_channel_provider_transition_stage() {
        let provider = TestNextFrameProvider::new(vec![]);
        let cfg = Arc::new(RollupConfig { holocene_time: Some(2), ..Default::default() });
        let mut channel_provider = ChannelProvider::new(cfg, provider);

        let active_stage = channel_provider.active_stage_mut().unwrap();

        // Update the L1 origin to Holocene activation.
        let ActiveStage::ChannelBank(stage) = active_stage else {
            panic!("Expected ChannelBank");
        };
        stage.prev.block_info = Some(BlockInfo { number: 1, timestamp: 2, ..Default::default() });

        // Transition to the ChannelAssembler stage.
        let active_stage = channel_provider.active_stage_mut().unwrap();
        let ActiveStage::ChannelAssembler(stage) = active_stage else {
            panic!("Expected ChannelAssembler");
        };

        assert_eq!(stage.origin().unwrap().number, 1);
    }
}
