#![no_std]
#![cfg_attr(any(target_arch = "mips", target_arch = "riscv64"), no_main)]

use kona_common::io;
use kona_common_proc::client_entry;

extern crate alloc;

#[client_entry(0xFFFFFFF)]
fn main() {
    io::print("Hello, world!\n");
}
