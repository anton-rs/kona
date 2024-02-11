//! This module contains the [ClientIO] struct, which is used to perform various IO operations
//! inside of the FPVM kernel within a `client` program.

use crate::{traits::BasicKernelInterface, types::RegisterSize};
use anyhow::Result;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "mips")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `MIPS32rel1` target architecture."]
        pub type ClientIO = crate::cannon::io::CannonIO;
    } else if #[cfg(target_arch = "riscv64")] {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `riscv64` target architecture."]
        pub type ClientIO = crate::asterisc::io::AsteriscIO;
    } else {
        #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `native` target architecture."]
        pub type ClientIO = native_io::NativeIO;
    }
}

/// File descriptors available to the `client` within the FPVM kernel.
#[derive(Debug, Clone, Copy)]
pub enum FileDescriptor {
    /// Read-only standard input stream.
    StdIn,
    /// Write-only standaard output stream.
    StdOut,
    /// Write-only standard error stream.
    StdErr,
    /// Read-only. Used to read the status of pre-image hinting.
    HintRead,
    /// Write-only. Used to provide pre-image hints
    HintWrite,
    /// Read-only. Used to read pre-images.
    PreimageRead,
    /// Write-only. Used to request pre-images.
    PreimageWrite,
    /// Other file descriptor, usually used for testing purposes.
    Wildcard(RegisterSize),
}

impl From<FileDescriptor> for RegisterSize {
    fn from(fd: FileDescriptor) -> Self {
        match fd {
            FileDescriptor::StdIn => 0,
            FileDescriptor::StdOut => 1,
            FileDescriptor::StdErr => 2,
            FileDescriptor::HintRead => 3,
            FileDescriptor::HintWrite => 4,
            FileDescriptor::PreimageRead => 5,
            FileDescriptor::PreimageWrite => 6,
            FileDescriptor::Wildcard(value) => value,
        }
    }
}
/// Print the passed string to the standard output [FileDescriptor].
#[inline]
pub fn print(s: &str) {
    ClientIO::write(FileDescriptor::StdOut, s.as_bytes()).expect("Error writing to stdout.");
}

/// Print the passed string to the standard error [FileDescriptor].
#[inline]
pub fn print_err(s: &str) {
    ClientIO::write(FileDescriptor::StdErr, s.as_bytes()).expect("Error writing to stderr.");
}

/// Write the passed buffer to the given [FileDescriptor].
#[inline]
pub fn write(fd: FileDescriptor, buf: &[u8]) -> Result<RegisterSize> {
    ClientIO::write(fd, buf)
}

/// Write the passed buffer to the given [FileDescriptor].
#[inline]
pub fn read(fd: FileDescriptor, buf: &mut [u8]) -> Result<RegisterSize> {
    ClientIO::read(fd, buf)
}

/// Exit the process with the given exit code.
#[inline]
pub fn exit(code: RegisterSize) -> ! {
    ClientIO::exit(code)
}

#[cfg(not(any(target_arch = "mips", target_arch = "riscv64")))]
mod native_io {
    extern crate std;

    use crate::{io::FileDescriptor, traits::BasicKernelInterface, types::RegisterSize};
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
        fn write(fd: FileDescriptor, buf: &[u8]) -> Result<RegisterSize> {
            let raw_fd: RegisterSize = fd.into();
            let mut file = unsafe { File::from_raw_fd(raw_fd as i32) };
            file.write_all(buf)
                .map_err(|e| anyhow!("Error writing to buffer to file descriptor: {e}"))?;

            // forget the file descriptor so that the `Drop` impl doesn't close it.
            std::mem::forget(file);

            Ok(0)
        }

        fn read(fd: FileDescriptor, buf: &mut [u8]) -> Result<RegisterSize> {
            let raw_fd: RegisterSize = fd.into();
            let mut file = unsafe { File::from_raw_fd(raw_fd as i32) };
            file.read(buf)
                .map_err(|e| anyhow!("Error reading from file descriptor: {e}"))?;

            // forget the file descriptor so that the `Drop` impl doesn't close it.
            std::mem::forget(file);

            Ok(0)
        }

        fn exit(code: RegisterSize) -> ! {
            std::process::exit(code as i32)
        }
    }
}
