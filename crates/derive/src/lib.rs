#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

/// Prelude exports common types and traits.
pub mod prelude {
    pub use super::{builder::DerivationPipeline, params::*};
    // pub use super::traits::prelude::*;
    // pub use super::types::prelude::*;
    // pub use super::stages::prelude::*;
    // pub use super::sources::prelude::*;
}

mod params;
pub use params::{
    ChannelID, CHANNEL_ID_LENGTH, CONFIG_UPDATE_EVENT_VERSION_0, CONFIG_UPDATE_TOPIC,
    DEPOSIT_EVENT_ABI, DEPOSIT_EVENT_ABI_HASH, DEPOSIT_EVENT_VERSION_0, DERIVATION_VERSION_0,
    FRAME_OVERHEAD, MAX_CHANNEL_BANK_SIZE, MAX_FRAME_LEN, MAX_RLP_BYTES_PER_CHANNEL,
    MAX_SPAN_BATCH_BYTES, SEQUENCER_FEE_VAULT_ADDRESS,
};

pub mod builder;
pub mod sources;
pub mod stages;
pub mod traits;
pub mod types;

#[cfg(feature = "online")]
mod online;
#[cfg(feature = "online")]
pub use online::prelude::*;
