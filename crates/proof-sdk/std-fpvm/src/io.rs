//! This module contains the `ClientIO` struct, which is a system call interface for the kernel.

use crate::{errors::IOResult, BasicKernelInterface, FileDescriptor};
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "mips")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `MIPS32rel1` target architecture."]
        pub(crate) type ClientIO = crate::mips32::io::Mips32IO;
    } else if #[cfg(target_arch = "riscv64")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `riscv64` target architecture."]
        pub(crate) type ClientIO = crate::riscv64::io::RiscV64IO;
    } else {
        #[doc = "No-op implementation of the [BasicKernelInterface] trait."]
        pub(crate) struct NoopClientIO;

        impl BasicKernelInterface for NoopClientIO {
            fn write(_: FileDescriptor, _: &[u8]) -> IOResult<usize> {
                Ok(0)
            }

            fn read(_: FileDescriptor, _: &mut [u8]) -> IOResult<usize> {
                Ok(0)
            }

            fn exit(code: usize) -> ! {
                std::process::exit(code as i32)
            }
        }

        #[doc = "No-op implementation of the [BasicKernelInterface] trait."]
        pub(crate) type ClientIO = NoopClientIO;
    }
}

/// Print the passed string to the standard output [FileDescriptor].
///
/// # Panics
/// Panics if the write operation fails.
#[inline]
pub fn print(s: &str) {
    ClientIO::write(FileDescriptor::StdOut, s.as_bytes()).expect("Error writing to stdout.");
}

/// Print the passed string to the standard error [FileDescriptor].
///
/// # Panics
/// Panics if the write operation fails.
#[inline]
pub fn print_err(s: &str) {
    ClientIO::write(FileDescriptor::StdErr, s.as_bytes()).expect("Error writing to stderr.");
}

/// Write the passed buffer to the given [FileDescriptor].
#[inline]
pub fn write(fd: FileDescriptor, buf: &[u8]) -> IOResult<usize> {
    ClientIO::write(fd, buf)
}

/// Write the passed buffer to the given [FileDescriptor].
#[inline]
pub fn read(fd: FileDescriptor, buf: &mut [u8]) -> IOResult<usize> {
    ClientIO::read(fd, buf)
}

/// Exit the process with the given exit code.
#[inline]
pub fn exit(code: usize) -> ! {
    ClientIO::exit(code)
}
