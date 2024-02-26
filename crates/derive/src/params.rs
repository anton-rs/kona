//! This module contains the parameters and identifying types for the derivation pipeline.

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
