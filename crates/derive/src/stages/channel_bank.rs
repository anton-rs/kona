//! This module contains the `ChannelBank` struct.

use alloc::collections::VecDeque;
use alloy_primitives::Bytes;
use hashbrown::HashMap;

use crate::{
    params::ChannelID,
    traits::{ChainProvider, DataAvailabilityProvider},
    types::{Channel, RollupConfig},
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
    // TODO
}
