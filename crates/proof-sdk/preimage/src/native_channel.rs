//! Native implementation of the [Channel] trait, backed by [tokio]'s [mpsc] channel primitives.
//!
//! [mpsc]: tokio::sync::mpsc

use crate::{
    errors::{ChannelError, ChannelResult},
    Channel,
};
use async_trait::async_trait;
use std::{io::Result, sync::Arc};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};

/// A bidirectional channel, allowing for synchronized communication between two parties.
#[derive(Debug)]
pub struct BidirectionalChannel {
    /// The client handle of the channel.
    pub client: NativeChannel,
    /// The host handle of the channel.
    pub host: NativeChannel,
}

impl BidirectionalChannel {
    /// Creates a [BidirectionalChannel] instance.
    pub fn new<const BUF: usize>() -> Result<Self> {
        let (bw, ar) = channel(BUF);
        let (aw, br) = channel(BUF);

        Ok(Self {
            client: NativeChannel { read: Arc::new(Mutex::new(ar)), write: aw },
            host: NativeChannel { read: Arc::new(Mutex::new(br)), write: bw },
        })
    }
}

/// A channel with a receiver and sender.
#[derive(Debug)]
pub struct NativeChannel {
    /// The receiver of the channel.
    pub(crate) read: Arc<Mutex<Receiver<Vec<u8>>>>,
    /// The sender of the channel.
    pub(crate) write: Sender<Vec<u8>>,
}

#[async_trait]
impl Channel for NativeChannel {
    async fn read(&self, buf: &mut [u8]) -> ChannelResult<usize> {
        let data = self.read.lock().await.recv().await.ok_or(ChannelError::Closed)?;
        let len = data.len().min(buf.len());
        buf[..len].copy_from_slice(&data[..len]);
        Ok(len)
    }

    async fn read_exact(&self, buf: &mut [u8]) -> ChannelResult<usize> {
        let mut read_lock = self.read.lock().await;

        let mut read = 0;
        while read < buf.len() {
            let data = read_lock.recv().await.ok_or(ChannelError::Closed)?;
            let len = data.len();

            if len + read > buf.len() {
                return Err(ChannelError::UnexpectedEOF);
            }

            buf[read..read + len].copy_from_slice(&data[..]);
            read += len;
        }

        Ok(read)
    }

    async fn write(&self, buf: &[u8]) -> ChannelResult<usize> {
        self.write.send(buf.to_vec()).await.unwrap();
        Ok(buf.len())
    }
}
