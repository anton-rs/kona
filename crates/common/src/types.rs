//! This module contains the local types for the `kona-common` crate.

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "mips")] {
        /// The size of the `mips32` target architecture's registers.
        pub type RegisterSize = u32;
    } else if #[cfg(target_arch = "riscv64")] {
        /// The size of the `riscv64` target architecture's registers.
        pub type RegisterSize = u64;
    } else {
        /// The size of the native target architecture's registers.
        pub type RegisterSize = u64;
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
