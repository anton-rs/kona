//! This module contains the [ChannelProvider] stage.

use super::{ChannelAssembler, ChannelBank, ChannelReaderProvider, NextFrameProvider};
use crate::{pipeline::PipelineResult, stages::multiplexed::multiplexed_stage};
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
        match self.active_stage_mut() {
            ActiveStage::ChannelAssembler(stage) => stage.next_data().await,
            ActiveStage::ChannelBank(stage) => stage.next_data().await,
        }
    }
}
