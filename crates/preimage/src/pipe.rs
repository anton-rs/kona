//! This module contains a rudamentary pipe between two file descriptors, using [kona_common::io] for
//! reading and writing from the file descriptors.

use anyhow::Result;
use kona_common::{io, FileDescriptor, RegisterSize};

/// A [ReadHandle] is a handle to read from one end of a [BidirectionalPipe].
#[derive(Debug, Clone, Copy)]
pub struct ReadHandle {
    pub(crate) read_fd: FileDescriptor,
}

impl ReadHandle {
    /// Create a new [ReadHandle] from a file descriptor.
    pub const fn new(read_fd: FileDescriptor) -> Self {
        Self { read_fd }
    }

    /// Read from the pipe into the given buffer.
    pub fn read(&self, buf: &mut [u8]) -> Result<RegisterSize> {
        io::read(self.read_fd, buf)
    }
}

/// A [WriteHandle] is a handle to write to one end of a [BidirectionalPipe].
#[derive(Debug, Clone, Copy)]
pub struct WriteHandle {
    write_fd: FileDescriptor,
}

impl WriteHandle {
    /// Create a new [WriteHandle] from a file descriptor.
    pub const fn new(write_fd: FileDescriptor) -> Self {
        Self { write_fd }
    }

    /// Write the given buffer to the pipe.
    pub fn write(&self, buf: &[u8]) -> Result<RegisterSize> {
        io::write(self.write_fd, buf)
    }
}

/// [PipeHandle] is a handle for one end of a bidirectional pipe.
#[derive(Debug, Clone, Copy)]
pub struct PipeHandle {
    /// File descriptor to read from
    read_handle: ReadHandle,
    /// File descriptor to write to
    write_handle: WriteHandle,
}

impl PipeHandle {
    /// Create a new [PipeHandle] from two file descriptors.
    pub const fn new(read_handle: ReadHandle, write_handle: WriteHandle) -> Self {
        Self {
            read_handle,
            write_handle,
        }
    }

    /// Read from the pipe into the given buffer.
    pub fn read(&self, buf: &mut [u8]) -> Result<RegisterSize> {
        self.read_handle.read(buf)
    }

    /// Write the given buffer to the pipe.
    pub fn write(&self, buf: &[u8]) -> Result<RegisterSize> {
        self.write_handle.write(buf)
    }
}
