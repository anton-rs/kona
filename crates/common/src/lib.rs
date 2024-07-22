#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(target_arch = "mips", feature(asm_experimental_arch))]
#![no_std]

extern crate alloc;

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

#[cfg(any(target_arch = "riscv64", target_os = "zkvm", target_os = "zkvm"))]
pub(crate) mod asterisc;
