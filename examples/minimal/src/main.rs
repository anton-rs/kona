#![no_std]
#![no_main]

use kona_common::io::{self, FileDescriptor};

extern crate alloc;

const HEAP_SIZE: usize = 0xFFFFFFF;

#[no_mangle]
pub extern "C" fn _start() {
    kona_common::alloc_heap!(HEAP_SIZE);
    io::print("Hello, world!\n");
    io::exit(0)
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("Panic: {}", info);
    let _ = io::write(FileDescriptor::StdErr, msg.as_bytes());
    io::exit(2)
}
