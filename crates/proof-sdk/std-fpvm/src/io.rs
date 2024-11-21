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
        use std::{fs::File, os::fd::FromRawFd, io::{Read, Write}};
        use crate::errors::IOError;

        #[doc = "Native implementation of the [BasicKernelInterface] trait."]
        pub(crate) struct NativeClientIO;

        impl BasicKernelInterface for NativeClientIO {
            fn write(fd: FileDescriptor, buf: &[u8]) -> IOResult<usize> {
                unsafe {
                    let mut file = File::from_raw_fd(fd as i32);
                    file.write_all(buf).map_err(|_| IOError(-9))?;
                    std::mem::forget(file);
                    Ok(buf.len())
                }
            }

            fn read(fd: FileDescriptor, buf: &mut [u8]) -> IOResult<usize> {
                unsafe {
                    let mut file = File::from_raw_fd(fd as i32);
                    file.read(buf).map_err(|_| IOError(-9))?;
                    std::mem::forget(file);
                    Ok(buf.len())
                }
            }

            fn exit(code: usize) -> ! {
                std::process::exit(code as i32)
            }
        }

        #[doc = "Native implementation of the [BasicKernelInterface] trait."]
        pub(crate) type ClientIO = NativeClientIO;
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
