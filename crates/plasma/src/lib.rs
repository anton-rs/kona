#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

// Re-export kona primitives.
pub use kona_primitives::*;

pub mod traits;
pub use traits::PlasmaInputFetcher;

pub mod types;
pub use types::{FinalizedHeadSignal, Keccak256Commitment, PlasmaError, SystemConfig};

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
