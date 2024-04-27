#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

pub mod chain_provider;
pub use chain_provider::ChainProvider;

pub mod l2_chain_provider;
pub use l2_chain_provider::L2ChainProvider;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
