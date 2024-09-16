//! Mock testing utilities for the [ChannelBank] stage.

use crate::{
    errors::{StageError, StageResult},
    stages::ChannelBankProvider,
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::{BlockInfo, Frame};

/// A mock [ChannelBankProvider] for testing the [ChannelBank] stage.
#[derive(Debug, Default)]
pub struct MockChannelBankProvider {
    /// The data to return.
    pub data: Vec<StageResult<Frame>>,
    /// The block info
    pub block_info: Option<BlockInfo>,
}

impl MockChannelBankProvider {
    /// Creates a new [MockChannelBankProvider] with the given data.
    pub fn new(data: Vec<StageResult<Frame>>) -> Self {
        Self { data, block_info: Some(BlockInfo::default()) }
    }
}

impl OriginProvider for MockChannelBankProvider {
    fn origin(&self) -> Option<BlockInfo> {
        self.block_info
    }
}

#[async_trait]
impl OriginAdvancer for MockChannelBankProvider {
    async fn advance_origin(&mut self) -> StageResult<()> {
        self.block_info = self.block_info.map(|mut bi| {
            bi.number += 1;
            bi
        });
        Ok(())
    }
}

#[async_trait]
impl ChannelBankProvider for MockChannelBankProvider {
    async fn next_frame(&mut self) -> StageResult<Frame> {
        self.data.pop().unwrap_or(Err(StageError::Eof))
    }
}

#[async_trait]
impl ResettableStage for MockChannelBankProvider {
    async fn reset(&mut self, _base: BlockInfo, _cfg: &SystemConfig) -> StageResult<()> {
        Ok(())
    }
}
