#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![no_std]

extern crate alloc;

pub mod l1;

pub mod l2;

pub mod sync;

pub mod errors;

pub mod executor;

mod hint;
pub use hint::HintType;

pub mod boot;
pub use boot::BootInfo;

mod caching_oracle;
pub use caching_oracle::{CachingOracle, FlushableCache};

mod blocking_runtime;
pub use blocking_runtime::block_on;
