#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

pub mod chain_provider;
pub mod l2_chain_provider;

/// A prelude that re-exports common types and traits.
pub mod prelude {
    pub use crate::chain_provider::ChainProvider;
    pub use crate::l2_chain_provider::L2ChainProvider;
}
