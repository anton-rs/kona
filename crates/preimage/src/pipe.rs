//! This module contains a rudamentary pipe between two file descriptors, using [kona_common::io] for
//! reading and writing from the file descriptors.

use anyhow::Result;
use kona_common::{
    io::{self, FileDescriptor},
    types::RegisterSize,
};
use spin::RwLock;

/// [BidirectionalPipe] is a spin-locked bidirectional pipe between two file descriptors. It may be shared between
/// multiple threads 
#[derive(Debug)]
pub struct BidirectionalPipe {
    a: RwLock<FileDescriptor>,
    b: RwLock<FileDescriptor>,
}

impl BidirectionalPipe {
    /// Create a new [BidirectionalPipe] from two file descriptors.
    pub const fn new(a: FileDescriptor, b: FileDescriptor) -> Self {
        Self {
            a: RwLock::new(a),
            b: RwLock::new(b),
        }
    }

    /// Get the first handle for the pipe. This handle can be used to read from file descriptor `a` and write to file 
    /// descriptor `b`.
    pub const fn handle_a(&self) -> PipeHandle<'_> {
        PipeHandle::new(ReadHandle { read_fd: &self.a }, WriteHandle { write_fd: &self.b })
    }

    /// Get the second handle for the pipe. This handle can be used to read from file descriptor `b` and write to file 
    /// descriptor `a`.
    pub const fn handle_b(&self) -> PipeHandle<'_> {
        PipeHandle::new(ReadHandle { read_fd: &self.b }, WriteHandle { write_fd: &self.a })
    }
}

/// A [ReadHandle] is a handle to read from one end of a [BidirectionalPipe].
#[derive(Debug)]
pub struct ReadHandle<'a> {
    read_fd: &'a RwLock<FileDescriptor>,
}

impl<'a> ReadHandle<'a> {
    /// Read from the pipe into the given buffer.
    pub fn read(&self, buf: &mut [u8]) -> Result<RegisterSize> {
        io::read(*self.read_fd.read(), buf)
    }
}

/// A [WriteHandle] is a handle to write to one end of a [BidirectionalPipe].
#[derive(Debug)]
pub struct WriteHandle<'a> {
    write_fd: &'a RwLock<FileDescriptor>,
}

impl<'a> WriteHandle<'a> {
    /// Write the given buffer to the pipe.
    pub fn write(&self, buf: &[u8]) -> Result<RegisterSize> {
        io::write(*self.write_fd.write(), buf)
    }
}

/// [PipeHandle] is a handle for one end of a bidirectional pipe.
#[derive(Debug)]
pub struct PipeHandle<'a> {
    /// File descriptor to read from
    read_handle: ReadHandle<'a>,
    /// File descriptor to write to
    write_handle: WriteHandle<'a>,
}

impl<'a> PipeHandle<'a> {
    /// Create a new [PipeHandle] from two file descriptors.
    pub const fn new(read_handle: ReadHandle<'a>, write_handle: WriteHandle<'a>) -> Self {
        Self { read_handle, write_handle }
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
