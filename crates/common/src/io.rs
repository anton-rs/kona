//! This module contains the [ClientIO] struct, which is used to perform various IO operations
//! inside of the FPVM kernel within a `client` program.

use crate::{BasicKernelInterface, FileDescriptor};
use anyhow::Result;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "mips")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `MIPS32rel1` target architecture."]
        pub type ClientIO = crate::cannon::io::CannonIO;
    } else if #[cfg(target_arch = "riscv64")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `riscv64` target architecture."]
        pub type ClientIO = crate::asterisc::io::AsteriscIO;
    } else if #[cfg(target_os = "zkvm")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `SP1` target architecture."]
        pub type ClientIO = crate::zkvm::io::ZkvmIO;
    } else {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `native` target architecture."]
        pub type ClientIO = native_io::NativeIO;
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
pub fn write(fd: FileDescriptor, buf: &[u8]) -> Result<usize> {
    ClientIO::write(fd, buf)
}

/// Write the passed buffer to the given [FileDescriptor].
#[inline]
pub fn read(fd: FileDescriptor, buf: &mut [u8]) -> Result<usize> {
    ClientIO::read(fd, buf)
}

/// Exit the process with the given exit code.
#[inline]
pub fn exit(code: usize) -> ! {
    ClientIO::exit(code)
}

#[cfg(not(any(target_arch = "mips", target_arch = "riscv64", target_os = "zkvm")))]
mod native_io {
    extern crate std;

    use crate::{io::FileDescriptor, traits::BasicKernelInterface};
    use anyhow::{anyhow, Result};
    use std::{
        fs::File,
        io::{Read, Write},
        os::fd::FromRawFd,
    };

    /// Mock IO implementation for native tests.
    #[derive(Debug)]
    pub struct NativeIO;

    impl BasicKernelInterface for NativeIO {
        fn write(fd: FileDescriptor, buf: &[u8]) -> Result<usize> {
            let raw_fd: usize = fd.into();
            let mut file = unsafe { File::from_raw_fd(raw_fd as i32) };

            file.write_all(buf)
                .map_err(|e| anyhow!("Error writing to buffer to file descriptor: {e}"))?;

            std::mem::forget(file);

            Ok(buf.len())
        }

        fn read(fd: FileDescriptor, buf: &mut [u8]) -> Result<usize> {
            let raw_fd: usize = fd.into();
            let mut file = unsafe { File::from_raw_fd(raw_fd as i32) };

            let n =
                file.read(buf).map_err(|e| anyhow!("Error reading from file descriptor: {e}"))?;

            std::mem::forget(file);

            Ok(n)
        }

        fn exit(code: usize) -> ! {
            std::process::exit(code as i32)
        }
    }
}
