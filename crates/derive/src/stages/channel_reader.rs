//! This module contains the implementation of the channel reader stage of the derivation pipelin.

use super::ChannelBank;
use crate::{
    traits::{ChainProvider, DataAvailabilityProvider},
    types::{BlockInfo, RollupConfig, StageResult},
};
use core::fmt::Debug;

type NextBatchFn = fn() -> StageResult<()>;

/// The [ChannelReader] stage is responsible for reading from the channel bank and decoding the
/// channel data into batches.
#[derive(Debug)]
pub struct ChannelReader<DAP, CP>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
{
    /// The rollup configuration.
    #[allow(dead_code)]
    cfg: RollupConfig,
    /// The previous stage of the derivation pipeline.
    prev: ChannelBank<DAP, CP>,
    /// The next batch function.
    next_batch: Option<NextBatchFn>,
}

impl<DAP, CP> ChannelReader<DAP, CP>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
{
    /// Create a new [ChannelReader] stage.
    pub fn new(cfg: RollupConfig, prev: ChannelBank<DAP, CP>) -> Self {
        Self {
            cfg,
            prev,
            next_batch: None,
        }
    }

    /// Returns the L1 origin [BlockInfo].
    pub fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }

    /// Writes to the channel
    pub fn write_channel(&mut self, _: &[u8]) {
        todo!()
    }

    /// Forces the read to continue with the next channel, resetting any
    /// decoding / decompression state to a fresh start.
    pub fn next_channel(&mut self) {
        self.next_batch = None;
    }
}
