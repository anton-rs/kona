//! This module contains a rudamentary pipe between two file descriptors, using [kona_common::io] for
//! reading and writing from the file descriptors.

use anyhow::Result;
use kona_common::{
    io::{self, FileDescriptor},
    types::RegisterSize,
};

/// [PipeHandle] is a handle for one end of a bidirectional pipe.
#[derive(Debug)]
pub struct PipeHandle {
    /// File descriptor to read from
    read_fd: FileDescriptor,
    /// File descriptor to write to
    write_fd: FileDescriptor,
}

impl PipeHandle {
    /// Create a new [PipeHandle] from two file descriptors.
    pub const fn new(read_fd: FileDescriptor, write_fd: FileDescriptor) -> Self {
        Self { read_fd, write_fd }
    }

    /// Read from the pipe into the given buffer.
    pub fn read(&self, buf: &mut [u8]) -> Result<RegisterSize> {
        io::read(self.read_fd, buf)
    }

    /// Write the given buffer to the pipe.
    pub fn write(&self, buf: &[u8]) -> Result<RegisterSize> {
        io::write(self.write_fd, buf)
    }
}

/// Creates a bidirectional pipe with four file descriptors.
pub fn create_bidirectional_pipe() -> Result<(PipeHandle, PipeHandle)> {
    Ok((
        PipeHandle::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite),
        PipeHandle::new(FileDescriptor::HintRead, FileDescriptor::HintWrite),
    ))
}
