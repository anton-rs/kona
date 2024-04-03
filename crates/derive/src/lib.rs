#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

mod params;
pub use params::{
    ChannelID, CHANNEL_ID_LENGTH, DERIVATION_VERSION_0, FRAME_OVERHEAD, MAX_CHANNEL_BANK_SIZE,
    MAX_RLP_BYTES_PER_CHANNEL, MAX_SPAN_BATCH_BYTES,
};

pub mod sources;
pub mod stages;
pub mod traits;
pub mod types;

/// The derivation pipeline is responsible for deriving L2 inputs from L1 data.
#[derive(Debug, Clone, Copy)]
pub struct DerivationPipeline;
