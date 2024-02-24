#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]
// Temp
#![allow(dead_code, unused, unreachable_pub)]

extern crate alloc;
extern crate std;

pub mod stages;
pub mod traits;
pub mod types;
