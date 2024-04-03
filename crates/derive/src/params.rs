//! This module contains the parameters and identifying types for the derivation pipeline.

use alloy_primitives::{b256, B256};

/// Count the tagging info as 200 in terms of buffer size.
pub const FRAME_OVERHEAD: usize = 200;

/// The version of the derivation pipeline.
pub const DERIVATION_VERSION_0: u8 = 0;

/// [MAX_SPAN_BATCH_BYTES] is the maximum amount of bytes that will be needed
/// to decode every span batch field. This value cannot be larger than
/// MaxRLPBytesPerChannel because single batch cannot be larger than channel size.
pub const MAX_SPAN_BATCH_BYTES: u64 = MAX_RLP_BYTES_PER_CHANNEL;

/// [MAX_RLP_BYTES_PER_CHANNEL] is the maximum amount of bytes that will be read from
/// a channel. This limit is set when decoding the RLP.
pub const MAX_RLP_BYTES_PER_CHANNEL: u64 = 10_000_000;

/// The maximum size of a channel bank.
pub const MAX_CHANNEL_BANK_SIZE: usize = 100_000_000;

/// [CHANNEL_ID_LENGTH] is the length of the channel ID.
pub const CHANNEL_ID_LENGTH: usize = 16;

/// [ChannelID] is an opaque identifier for a channel.
pub type ChannelID = [u8; CHANNEL_ID_LENGTH];

/// `keccak256("ConfigUpdate(uint256,uint8,bytes)")`
pub const CONFIG_UPDATE_TOPIC: B256 =
    b256!("1d2b0bda21d56b8bd12d4f94ebacffdfb35f5e226f84b461103bb8beab6353be");

/// The initial version of the system config event log.
pub const CONFIG_UPDATE_EVENT_VERSION_0: B256 = B256::ZERO;

/// Frames cannot be larger than 1MB.
/// Data transactions that carry frames are generally not larger than 128 KB due to L1 network conditions,
/// but we leave space to grow larger anyway (gas limit allows for more data).
pub const MAX_FRAME_LEN: usize = 1000;
