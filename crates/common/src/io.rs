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
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FileDescriptor {
    /// Read-only standard input stream.
    StdIn = 0,
    /// Write-only standaard output stream.
    StdOut = 1,
    /// Write-only standard error stream.
    StdErr = 2,
    /// Read-only. Used to read the status of pre-image hinting.
    HintRead = 3,
    /// Write-only. Used to provide pre-image hints
    HintWrite = 4,
    /// Read-only. Used to read pre-images.
    PreimageRead = 5,
    /// Write-only. Used to request pre-images.
    PreimageWrite = 6,
}

/// Print the passed string to the standard output [FileDescriptor].
#[inline]
pub fn print(s: &str) {
    ClientIO::write(FileDescriptor::StdOut, s.as_bytes()).expect("Error writing to stdout.");
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
            let mut file = unsafe { File::from_raw_fd(fd as i32) };
            file.write_all(buf)
                .map_err(|e| anyhow!("Error writing to buffer to file descriptor: {e}"))?;

            // forget the file descriptor so that the `Drop` impl doesn't close it.
            std::mem::forget(file);

            Ok(0)
        }

        fn read(fd: FileDescriptor, buf: &mut [u8]) -> Result<RegisterSize> {
            let mut file = unsafe { File::from_raw_fd(fd as i32) };
            file.read_exact(buf)
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
