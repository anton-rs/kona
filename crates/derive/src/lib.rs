#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::fmt::Debug;
use traits::ChainProvider;
use types::RollupConfig;

mod params;
pub use params::{
    ChannelID, CHANNEL_ID_LENGTH, CONFIG_UPDATE_EVENT_VERSION_0, CONFIG_UPDATE_TOPIC,
    DEPOSIT_EVENT_ABI, DEPOSIT_EVENT_ABI_HASH, DEPOSIT_EVENT_VERSION_0, DERIVATION_VERSION_0,
    FRAME_OVERHEAD, MAX_CHANNEL_BANK_SIZE, MAX_FRAME_LEN, MAX_RLP_BYTES_PER_CHANNEL,
    MAX_SPAN_BATCH_BYTES, SEQUENCER_FEE_VAULT_ADDRESS,
};

pub mod sources;
pub mod stages;
pub mod traits;
pub mod types;

#[cfg(feature = "online")]
pub mod alloy_providers;

/// The derivation pipeline is responsible for deriving L2 inputs from L1 data.
#[derive(Debug, Clone, Copy)]
pub struct DerivationPipeline;

impl DerivationPipeline {
    /// Creates a new instance of the [DerivationPipeline].
    pub fn new<P>(_rollup_config: Arc<RollupConfig>, _chain_provider: P) -> Self
    where
        P: ChainProvider + Clone + Debug + Send,
    {
        unimplemented!("TODO: High-level pipeline composition helper.")
    }
}
