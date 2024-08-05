//! This module contains the local types for the `kona-common` crate.

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
    /// Other file descriptor.
    Wildcard(usize),
}

impl From<FileDescriptor> for usize {
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

impl From<FileDescriptor> for i32 {
    fn from(fd: FileDescriptor) -> Self {
        usize::from(fd) as i32
    }
}
