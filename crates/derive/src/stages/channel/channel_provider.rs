//! This module contains the [ChannelProvider] stage.

use super::{ChannelAssembler, ChannelBank, ChannelReaderProvider, NextFrameProvider};
use crate::{
    pipeline::{OriginAdvancer, PipelineResult, ResettableStage},
    prelude::OriginProvider,
    stages::multiplexed::multiplexed_stage,
};
use alloc::{boxed::Box, sync::Arc};
use alloy_primitives::Bytes;
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::BlockInfo;

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
