#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(target_arch = "mips", feature(asm_experimental_arch))]
#![no_std]

pub mod io;
pub mod malloc;
pub mod traits;
pub mod types;

#[cfg(target_arch = "mips")]
pub(crate) mod cannon;

#[cfg(target_arch = "riscv64")]
pub(crate) mod asterisc;
