//! This module contains the `ChannelBank` struct.

use alloc::collections::VecDeque;
use alloy_primitives::Bytes;
use anyhow::{anyhow, Result};
use hashbrown::HashMap;

use crate::{
    params::{ChannelID, MAX_CHANNEL_BANK_SIZE},
    traits::{ChainProvider, DataAvailabilityProvider},
    types::{BlockInfo, Channel, Frame, RollupConfig},
};

use super::l1_retrieval::L1Retrieval;

/// [ChannelBank] is a stateful stage that does the following:
/// 1. Unmarshalls frames from L1 transaction data
/// 2. Applies those frames to a channel
/// 3. Attempts to read from the channel when it is ready
/// 4. Prunes channels (not frames) when the channel bank is too large.
///
/// Note: we prune before we ingest data.
/// As we switch between ingesting data & reading, the prune step occurs at an odd point
/// Specifically, the channel bank is not allowed to become too large between successive calls
/// to `IngestData`. This means that we can do an ingest and then do a read while becoming too large.
/// [ChannelBank] buffers channel frames, and emits full channel data
pub struct ChannelBank<DAP, CP>
where
    DAP: DataAvailabilityProvider,
    CP: ChainProvider,
{
    /// The rollup configuration.
    cfg: RollupConfig,
    /// Map of channels by ID.
    channels: HashMap<ChannelID, Channel>,
    /// Channels in FIFO order.
    channel_queue: VecDeque<ChannelID>,
    /// The previous stage of the derivation pipeline.
    prev: L1Retrieval<DAP, CP>,
    /// Chain provider.
    chain_provider: CP,
}

impl<DAP, CP> ChannelBank<DAP, CP>
where
    DAP: DataAvailabilityProvider,
    CP: ChainProvider,
{
    /// Create a new [ChannelBank] stage.
    pub fn new(cfg: RollupConfig, prev: L1Retrieval<DAP, CP>, chain_provider: CP) -> Self {
        Self {
            cfg,
            channels: HashMap::new(),
            channel_queue: VecDeque::new(),
            prev,
            chain_provider,
        }
    }

    /// Returns the L1 origin [BlockInfo].
    pub fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }

    /// Prunes the Channel bank, until it is below [MAX_CHANNEL_BANK_SIZE].
    pub fn prune(&mut self) -> Result<()> {
        // Check total size
        let mut total_size = self.channels.iter().fold(0, |acc, (_, c)| acc + c.size());
        // Prune until it is reasonable again. The high-priority channel failed to be read,
        // so we prune from there.
        while total_size > MAX_CHANNEL_BANK_SIZE {
            let id = self
                .channel_queue
                .pop_front()
                .ok_or(anyhow!("No channel to prune"))?;
            let channel = self
                .channels
                .remove(&id)
                .ok_or(anyhow!("Could not find channel"))?;
            total_size -= channel.size();
        }
        Ok(())
    }

    /// Adds new L1 data to the channel bank. Should only be called after all data has been read.
    pub fn ingest_frame(&mut self, frame: Frame) -> Result<()> {
        let origin = *self.origin().ok_or(anyhow!("No origin"))?;

        let current_channel = self.channels.entry(frame.id).or_insert_with(|| {
            // Create a new channel
            let channel = Channel::new(frame.id, origin);
            self.channel_queue.push_back(frame.id);
            channel
        });

        // Check if the channel is not timed out. If it has, ignore the frame.
        if current_channel.open_block_number() + self.cfg.channel_timeout < origin.number {
            return Ok(());
        }

        // Ingest the frame. If it fails, ignore the frame.
        if current_channel.add_frame(frame, origin).is_err() {
            return Ok(());
        }

        self.prune()
    }
}
