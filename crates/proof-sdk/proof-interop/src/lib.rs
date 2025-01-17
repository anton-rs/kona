#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(any(test, feature = "arbitrary")), no_std)]

extern crate alloc;

pub mod pre_state;

mod hint;
pub use hint::{Hint, HintType};

pub mod boot;
pub use boot::BootInfo;
