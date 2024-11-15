//! This module contains a rudamentary pipe between two file descriptors, using [kona_common::io]
//! for reading and writing from the file descriptors.

use alloc::boxed::Box;
use async_trait::async_trait;
use core::{
    cell::RefCell,
    cmp::Ordering,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use kona_common::{errors::IOResult, io, FileDescriptor};
use kona_preimage::{
    errors::{ChannelError, ChannelResult},
    Channel,
};

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
        Self { read_handle, write_handle }
    }

    /// Read from the pipe into the given buffer.
    pub fn read(&self, buf: &mut [u8]) -> IOResult<usize> {
        io::read(self.read_handle, buf)
    }

    /// Reads exactly `buf.len()` bytes into `buf`.
    pub fn read_exact<'a>(&self, buf: &'a mut [u8]) -> impl Future<Output = IOResult<usize>> + 'a {
        ReadFuture { pipe_handle: *self, buf: RefCell::new(buf), read: 0 }
    }

    /// Write the given buffer to the pipe.
    pub fn write<'a>(&self, buf: &'a [u8]) -> impl Future<Output = IOResult<usize>> + 'a {
        WriteFuture { pipe_handle: *self, buf, written: 0 }
    }

    /// Returns the read handle for the pipe.
    pub const fn read_handle(&self) -> FileDescriptor {
        self.read_handle
    }

    /// Returns the write handle for the pipe.
    pub const fn write_handle(&self) -> FileDescriptor {
        self.write_handle
    }
}

#[async_trait]
impl Channel for PipeHandle {
    async fn read(&self, buf: &mut [u8]) -> ChannelResult<usize> {
        self.read(buf).map_err(|_| ChannelError::Closed)
    }

    async fn read_exact(&self, buf: &mut [u8]) -> ChannelResult<usize> {
        self.read_exact(buf).await.map_err(|_| ChannelError::Closed)
    }

    async fn write(&self, buf: &[u8]) -> ChannelResult<usize> {
        self.write(buf).await.map_err(|_| ChannelError::Closed)
    }
}

/// A future that reads from a pipe, returning [Poll::Ready] when the buffer is full.
struct ReadFuture<'a> {
    /// The pipe handle to read from
    pipe_handle: PipeHandle,
    /// The buffer to read into
    buf: RefCell<&'a mut [u8]>,
    /// The number of bytes read so far
    read: usize,
}

impl Future for ReadFuture<'_> {
    type Output = IOResult<usize>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut buf = self.buf.borrow_mut();
        let buf_len = buf.len();
        let chunk_read = self.pipe_handle.read(&mut buf[self.read..])?;

        // Drop the borrow on self.
        drop(buf);

        self.read += chunk_read;

        match self.read.cmp(&buf_len) {
            Ordering::Greater | Ordering::Equal => Poll::Ready(Ok(self.read)),
            Ordering::Less => {
                // Register the current task to be woken up when it can make progress
                ctx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

/// A future that writes to a pipe, returning [Poll::Ready] when the full buffer has been written.
struct WriteFuture<'a> {
    /// The pipe handle to write to
    pipe_handle: PipeHandle,
    /// The buffer to write
    buf: &'a [u8],
    /// The number of bytes written so far
    written: usize,
}

impl Future for WriteFuture<'_> {
    type Output = IOResult<usize>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        match io::write(self.pipe_handle.write_handle(), &self.buf[self.written..]) {
            Ok(0) => Poll::Ready(Ok(self.written)), // Finished writing
            Ok(n) => {
                self.written += n;

                if self.written >= self.buf.len() {
                    return Poll::Ready(Ok(self.written));
                }

                // Register the current task to be woken up when it can make progress
                ctx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_read_handle() {
        let read_handle = FileDescriptor::StdIn;
        let write_handle = FileDescriptor::StdOut;
        let pipe_handle = PipeHandle::new(read_handle, write_handle);
        let ref_read_handle = pipe_handle.read_handle();
        assert_eq!(read_handle, ref_read_handle);
    }
}
