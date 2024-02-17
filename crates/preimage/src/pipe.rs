//! This module contains a rudamentary pipe between two file descriptors, using [kona_common::io] for
//! reading and writing from the file descriptors.

use anyhow::{bail, Result};
use kona_common::{io, FileDescriptor, RegisterSize};

/// [PipeHandle] is a handle for one end of a bidirectional pipe.
#[derive(Debug, Clone, Copy)]
pub struct PipeHandle {
    /// File descriptor to read from
    read_handle: FileDescriptor,
    /// File descriptor to write to
    write_handle: FileDescriptor,
}

impl PipeHandle {
    /// Create a new [PipeHandle] from two file descriptors.
    pub const fn new(read_handle: FileDescriptor, write_handle: FileDescriptor) -> Self {
        Self {
            read_handle,
            write_handle,
        }
    }

    /// Read from the pipe into the given buffer.
    pub fn read(&self, buf: &mut [u8]) -> Result<RegisterSize> {
        io::read(self.read_handle, buf)
    }

    /// Reads exactly `buf.len()` bytes into `buf`, blocking until all bytes are read.
    pub fn read_exact(&self, buf: &mut [u8]) -> Result<RegisterSize> {
        let mut read = 0;
        while read < buf.len() {
            let chunk_read = self.read(&mut buf[read..])?;
            read += chunk_read as usize;
        }
        Ok(read as RegisterSize)
    }

    /// Write the given buffer to the pipe.
    pub fn write(&self, buf: &[u8]) -> Result<RegisterSize> {
        let mut written = 0;
        loop {
            match io::write(self.write_handle, &buf[written..]) {
                Ok(0) => break,
                Ok(n) => {
                    written += n as usize;
                    continue;
                }
                Err(e) => bail!("Failed to write preimage key: {}", e),
            }
        }
        Ok(written as RegisterSize)
    }
}
