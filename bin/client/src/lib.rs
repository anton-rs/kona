#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

pub mod l1;
pub use l1::OracleL1ChainProvider;

pub mod l2;
pub use l2::OracleL2ChainProvider;

mod hint;
pub use hint::HintType;

pub mod boot;
pub use boot::BootInfo;

mod caching_oracle;
pub use caching_oracle::{CachingOracle, FlushableCache};
