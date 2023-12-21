#![no_std]
#![no_main]

use kona_common::{
    io::{ClientIO, FileDescriptor},
    traits::BasicKernelInterface,
};

extern crate alloc;

const HEAP_SIZE: usize = 0xFFFF;

#[no_mangle]
pub extern "C" fn _start() {
    kona_common::alloc_heap!(HEAP_SIZE);

    let _ = ClientIO::write(FileDescriptor::StdOut, b"Hello, world!\n");

    ClientIO::exit(0)
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("Panic: {}", info);
    let _ = ClientIO::write(FileDescriptor::StdErr, msg.as_bytes());
    ClientIO::exit(2)
}
