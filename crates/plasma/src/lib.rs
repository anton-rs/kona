#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

pub mod traits;
pub mod types;

// Re-export kona primitives.
pub use kona_primitives::prelude::*;

/// The prelude exports common types and traits.
pub mod prelude {
    pub use crate::{
        traits::{ChainProvider, PlasmaInputFetcher},
        types::{FinalizedHeadSignal, Keccak256Commitment, PlasmaError, SystemConfig},
    };
}

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
