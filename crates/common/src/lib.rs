#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(target_arch = "mips", feature(asm_experimental_arch))]
#![no_std]

extern crate alloc;

#[cfg(not(feature = "no-io"))]
pub mod io;

pub mod malloc;

mod traits;
pub use traits::BasicKernelInterface;

mod types;
pub use types::FileDescriptor;

mod executor;
pub use executor::block_on;

#[cfg(target_arch = "mips")]
pub(crate) mod cannon;

#[cfg(target_arch = "riscv64")]
pub(crate) mod asterisc;
