#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(any(test, feature = "metrics")), no_std)]

extern crate alloc;

mod macros;

pub mod batch;
pub mod errors;
pub mod params;
pub mod pipeline;
pub mod sources;
pub mod stages;
pub mod traits;

#[cfg(feature = "online")]
pub mod online;

#[cfg(feature = "metrics")]
pub mod metrics;
