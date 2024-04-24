#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

pub mod block;
pub mod genesis;
pub mod params;
pub mod rollup_config;
pub mod system_config;

/// The prelude exports common types and traits.
pub mod prelude {
    pub use crate::{
        block::{Block, BlockID, BlockInfo, BlockKind, L2BlockInfo, OpBlock, Withdrawal},
        genesis::Genesis,
        rollup_config::RollupConfig,
        system_config::{SystemAccounts, SystemConfig, SystemConfigUpdateType},
    };
}
