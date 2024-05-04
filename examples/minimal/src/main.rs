#![no_std]
#![cfg_attr(any(target_arch = "mips", target_arch = "riscv64"), no_main)]

use kona_common::io;

extern crate alloc;

#[allow(dead_code)]
const HEAP_SIZE: usize = 0xFFFFFFF;

fn main() {
    kona_common::alloc_heap!(HEAP_SIZE);
    io::print("Hello, world!\n");
    io::exit(0)
}

#[cfg(any(target_arch = "mips", target_arch = "riscv64"))]
#[no_mangle]
pub extern "C" fn _start() {
    main()
}

#[cfg(any(target_arch = "mips", target_arch = "riscv64"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("Panic: {}", info);
    let _ = io::print_err(msg.as_ref());
    io::exit(2)
}
