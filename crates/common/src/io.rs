//! This module contains the `ClientIO` struct, which is a system call interface for the kernel.

use crate::{errors::IOResult, BasicKernelInterface, FileDescriptor};
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "mips")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `MIPS32rel1` target architecture."]
        pub(crate) type ClientIO = crate::mips32::io::Mips32IO;
    } else if #[cfg(target_arch = "riscv64")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `riscv64` target architecture."]
        pub(crate)  type ClientIO = crate::riscv64::io::RiscV64IO;
    } else if #[cfg(target_os = "zkvm")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `SP1` target architecture."]
        pub(crate) type ClientIO = crate::zkvm::io::ZkvmIO;
    } else {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `native` target architecture."]
        pub(crate) type ClientIO = native_io::NativeIO<'static>;
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

/// Native IO Module
#[cfg(not(any(target_arch = "mips", target_arch = "riscv64", target_os = "zkvm")))]
pub(crate) mod native_io {
    use crate::{
        errors::{IOError, IOResult},
        io::FileDescriptor,
        traits::BasicKernelInterface,
    };
    use std::{
        fs::File,
        io::{Read, Write},
        os::fd::FromRawFd,
    };

    /// Mock IO implementation for native tests.
    #[derive(Debug)]
    pub(crate) struct NativeIO;

    impl BasicKernelInterface for NativeIO {
        fn write(fd: FileDescriptor, buf: &[u8]) -> IOResult<usize> {
            let raw_fd: usize = fd.into();
            let mut file = unsafe { File::from_raw_fd(raw_fd as i32) };

            file.write_all(buf).map_err(|_| IOError(9))?;

            std::mem::forget(file);

            Ok(buf.len())
        }

        fn read(fd: FileDescriptor, buf: &mut [u8]) -> IOResult<usize> {
            let raw_fd: usize = fd.into();
            let mut file = unsafe { File::from_raw_fd(raw_fd as i32) };

            let n = file.read(buf).map_err(|_| IOError(9))?;

            std::mem::forget(file);

            Ok(n)
        }

        fn exit(code: usize) -> ! {
            std::process::exit(code as i32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::FileDescriptor;

    #[test]
    fn test_print() {
        print("Hello, World!");
    }

    #[test]
    fn test_print_err() {
        print_err("Hello, World!");
    }

    #[test]
    fn test_write() {
        let buf = b"Hello, World!";
        write(FileDescriptor::StdOut, buf).unwrap();
    }

    #[test]
    fn test_read() {
        let mut buf = [0u8; 1024];
        read(FileDescriptor::StdIn, &mut buf).unwrap();
    }

    #[test]
    fn test_exit() {
        exit(0);
    }
}
