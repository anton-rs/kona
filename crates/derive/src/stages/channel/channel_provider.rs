//! This module contains the [ChannelProvider] stage.

use super::{ChannelAssembler, ChannelBank, ChannelReaderProvider, NextFrameProvider};
use crate::stages::multiplexed::multiplexed_stage;
use alloy_primitives::Bytes;
use core::fmt::Debug;

multiplexed_stage!(
    ChannelProvider<NextFrameProvider>
    stages: {
        ChannelAssembler => is_holocene_active,
    }
    default_stage: ChannelBank
);

#[async_trait]
impl<P> ChannelReaderProvider for ChannelProvider<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + SignalReceiver + Send + Debug,
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
    use crate::{
        prelude::{OriginProvider, PipelineError},
        stages::ChannelReaderProvider,
        test_utils::TestNextFrameProvider,
        traits::{ResetSignal, SignalReceiver},
    };
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
    fn test_channel_provider_retain_current_bank() {
        let provider = TestNextFrameProvider::new(vec![]);
        let cfg = Arc::new(RollupConfig::default());
        let mut channel_provider = ChannelProvider::new(cfg, provider);

        // Assert the multiplexer hasn't been initialized.
        assert!(channel_provider.active_stage.is_none());
        assert!(channel_provider.prev.is_some());

        // Load in the active stage.
        assert!(matches!(
            channel_provider.active_stage_mut().unwrap(),
            ActiveStage::ChannelBank(_)
        ));
        // Ensure the active stage is retained on the second call.
        assert!(matches!(
            channel_provider.active_stage_mut().unwrap(),
            ActiveStage::ChannelBank(_)
        ));
    }

    #[test]
    fn test_channel_provider_retain_current_assembler() {
        let provider = TestNextFrameProvider::new(vec![]);
        let cfg = Arc::new(RollupConfig { holocene_time: Some(0), ..Default::default() });
        let mut channel_provider = ChannelProvider::new(cfg, provider);

        // Assert the multiplexer hasn't been initialized.
        assert!(channel_provider.active_stage.is_none());
        assert!(channel_provider.prev.is_some());

        // Load in the active stage.
        assert!(matches!(
            channel_provider.active_stage_mut().unwrap(),
            ActiveStage::ChannelAssembler(_)
        ));
        // Ensure the active stage is retained on the second call.
        assert!(matches!(
            channel_provider.active_stage_mut().unwrap(),
            ActiveStage::ChannelAssembler(_)
        ));
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
        assert!(matches!(active_stage, ActiveStage::ChannelAssembler(_)));

        assert_eq!(channel_provider.origin().unwrap().number, 1);
    }

    #[test]
    fn test_channel_provider_transition_stage_backwards() {
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
            panic!("Expected ChannelBank");
        };

        // Update the L1 origin to before Holocene activation, to simulate a re-org.
        stage.prev.block_info = Some(BlockInfo::default());

        let active_stage = channel_provider.active_stage_mut().unwrap();
        assert!(matches!(active_stage, ActiveStage::ChannelBank(_)));
    }

    #[tokio::test]
    async fn test_channel_provider_reset_bank() {
        let frames = [
            crate::frame!(0xFF, 0, vec![0xDD; 50], false),
            crate::frame!(0xFF, 1, vec![0xDD; 50], true),
        ];
        let provider = TestNextFrameProvider::new(frames.into_iter().rev().map(Ok).collect());
        let cfg = Arc::new(RollupConfig::default());
        let mut channel_provider = ChannelProvider::new(cfg.clone(), provider);

        // Load in the first frame.
        assert_eq!(
            channel_provider.next_data().await.unwrap_err(),
            PipelineError::NotEnoughData.temp()
        );
        let Ok(ActiveStage::ChannelBank(channel_bank)) = channel_provider.active_stage_mut() else {
            panic!("Expected ChannelBank");
        };
        // Ensure a channel is in the queue.
        assert!(channel_bank.channel_queue.len() == 1);

        // Reset the channel provider.
        channel_provider.signal(ResetSignal::default().signal()).await.unwrap();

        // Ensure the channel queue is empty after reset.
        let Ok(ActiveStage::ChannelBank(channel_bank)) = channel_provider.active_stage_mut() else {
            panic!("Expected ChannelBank");
        };
        assert!(channel_bank.channel_queue.is_empty());
    }

    #[tokio::test]
    async fn test_channel_provider_reset_assembler() {
        let frames = [
            crate::frame!(0xFF, 0, vec![0xDD; 50], false),
            crate::frame!(0xFF, 1, vec![0xDD; 50], true),
        ];
        let provider = TestNextFrameProvider::new(frames.into_iter().rev().map(Ok).collect());
        let cfg = Arc::new(RollupConfig { holocene_time: Some(0), ..Default::default() });
        let mut channel_provider = ChannelProvider::new(cfg.clone(), provider);

        // Load in the first frame.
        assert_eq!(
            channel_provider.next_data().await.unwrap_err(),
            PipelineError::NotEnoughData.temp()
        );
        let Ok(ActiveStage::ChannelAssembler(channel_assembler)) =
            channel_provider.active_stage_mut()
        else {
            panic!("Expected ChannelBank");
        };
        // Ensure a channel is being built.
        assert!(channel_assembler.channel.is_some());

        // Reset the channel provider.
        channel_provider.signal(ResetSignal::default().signal()).await.unwrap();

        // Ensure the channel assembler is empty after reset.
        let Ok(ActiveStage::ChannelAssembler(channel_assembler)) =
            channel_provider.active_stage_mut()
        else {
            panic!("Expected ChannelBank");
        };
        assert!(channel_assembler.channel.is_none());
    }
}
