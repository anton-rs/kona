//! Test utilities for the `kona-preimage` crate.

use os_pipe::{PipeReader, PipeWriter};
use std::io::Result;

/// A bidirectional pipe, with a client and host end.
#[derive(Debug)]
pub(crate) struct BidirectionalPipe {
    pub(crate) client: Pipe,
    pub(crate) host: Pipe,
}

/// A single-direction pipe, with a read and write end.
#[derive(Debug)]
pub(crate) struct Pipe {
    pub(crate) read: PipeReader,
    pub(crate) write: PipeWriter,
}

/// Creates a [BidirectionalPipe] instance.
pub(crate) fn bidirectional_pipe() -> Result<BidirectionalPipe> {
    let (ar, bw) = os_pipe::pipe()?;
    let (br, aw) = os_pipe::pipe()?;

    Ok(BidirectionalPipe {
        client: Pipe { read: ar, write: aw },
        host: Pipe { read: br, write: bw },
    })
}
