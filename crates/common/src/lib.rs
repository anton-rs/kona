#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(target_arch = "mips", feature(asm_experimental_arch))]
#![cfg_attr(any(target_arch = "mips", target_arch = "riscv64", target_os = "zkvm"), no_std)]

extern crate alloc;

pub mod errors;

pub mod io;

pub mod malloc;

mod traits;
pub use traits::BasicKernelInterface;

mod types;
pub use types::FileDescriptor;

mod executor;
pub use executor::block_on;

pub(crate) mod linux;

#[cfg(target_arch = "mips")]
pub(crate) mod cannon;

#[cfg(target_arch = "riscv64")]
pub(crate) mod asterisc;

#[cfg(target_os = "zkvm")]
pub(crate) mod zkvm;
