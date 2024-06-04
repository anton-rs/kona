#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

mod params;
pub use params::{
    ChannelID, CHANNEL_ID_LENGTH, CONFIG_UPDATE_EVENT_VERSION_0, CONFIG_UPDATE_TOPIC,
    DERIVATION_VERSION_0, FRAME_OVERHEAD, MAX_CHANNEL_BANK_SIZE, MAX_FRAME_LEN,
    MAX_RLP_BYTES_PER_CHANNEL, MAX_SPAN_BATCH_BYTES, SEQUENCER_FEE_VAULT_ADDRESS,
};

pub mod pipeline;
pub mod sources;
pub mod stages;
pub mod traits;
pub mod types;

#[cfg(feature = "online")]
pub mod online;
#[cfg(feature = "online")]
pub use online::new_online_stack;
