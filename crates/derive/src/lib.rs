#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::fmt::Debug;
use kona_primitives::rollup_config::RollupConfig;
use traits::ChainProvider;

pub mod errors;
pub mod sources;
pub mod stages;
pub mod traits;

#[cfg(feature = "online")]
mod online;
#[cfg(feature = "online")]
pub use online::prelude::*;

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
