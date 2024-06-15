#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]
#![cfg_attr(any(target_arch = "mips", target_arch = "riscv64"), no_main)]

use kona_client::{BootInfo, CachingOracle};
use kona_common::io;
use kona_common_proc::client_entry;

extern crate alloc;

/// The size of the LRU cache in the oracle.
const ORACLE_LRU_SIZE: usize = 16;

#[client_entry(0x77359400)]
fn main() -> Result<()> {
    kona_common::block_on(async move {
        let caching_oracle = CachingOracle::new(ORACLE_LRU_SIZE);
        let boot = BootInfo::load(&caching_oracle).await?;
        io::print(&alloc::format!("{:?}\n", boot));
        Ok::<_, anyhow::Error>(())
    })
}
