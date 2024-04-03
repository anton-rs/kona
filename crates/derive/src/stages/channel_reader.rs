//! This module contains the `ChannelReader` struct.

use super::channel_bank::ChannelBank;
use crate::{
    traits::{ChainProvider, DataAvailabilityProvider},
    types::{Batch, BlockInfo, StageError, StageResult},
};
use alloc::vec::Vec;
use anyhow::anyhow;
use core::fmt::Debug;
use miniz_oxide::inflate::decompress_to_vec;

/// [ChannelReader] is a stateful stage that does the following:
#[derive(Debug)]
pub struct ChannelReader<DAP, CP>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
{
    /// The previous stage of the derivation pipeline.
    prev: ChannelBank<DAP, CP>,
    /// The batch reader.
    next_batch: Option<BatchReader>,
}

impl<DAP, CP> ChannelReader<DAP, CP>
where
    DAP: DataAvailabilityProvider + Debug,
    CP: ChainProvider + Debug,
{
    /// Create a new [ChannelReader] stage.
    pub fn new(prev: ChannelBank<DAP, CP>) -> Self {
        Self {
            prev,
            next_batch: None,
        }
    }

    /// Pulls out the next Batch from the available channel.
    pub async fn next_batch(&mut self) -> StageResult<Batch> {
        if let Err(e) = self.set_batch_reader().await {
            self.next_channel();
            return Err(e);
        }
        match self
            .next_batch
            .as_mut()
            .unwrap()
            .next_batch()
            .ok_or(StageError::NotEnoughData)
        {
            Ok(batch) => Ok(batch),
            Err(e) => {
                self.next_channel();
                Err(e)
            }
        }
    }

    /// Creates the batch reader from available channel data.
    async fn set_batch_reader(&mut self) -> StageResult<()> {
        if self.next_batch.is_none() {
            let channel = match self.prev.next_data().await {
                Ok(Some(channel)) => channel,
                Ok(None) => return Err(anyhow!("no channel").into()),
                Err(err) => return Err(err),
            };
            self.next_batch = Some(BatchReader::from(&channel[..]));
        }
        Ok(())
    }

    /// Returns the L1 origin [BlockInfo].
    pub fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }

    /// Forces the read to continue with the next channel, resetting any
    /// decoding / decompression state to a fresh start.
    pub fn next_channel(&mut self) {
        self.next_batch = None;
    }
}

/// Batch Reader provides a function that iteratively consumes batches from the reader.
/// The L1Inclusion block is also provided at creation time.
/// Warning: the batch reader can read every batch-type.
/// The caller of the batch-reader should filter the results.
#[derive(Debug)]
pub(crate) struct BatchReader {
    /// The raw data to decode.
    data: Option<Vec<u8>>,
    /// Decompressed data.
    decompressed: Vec<u8>,
}

impl BatchReader {
    /// Pulls out the next batch from the reader.
    pub(crate) fn next_batch(&mut self) -> Option<Batch> {
        if let Some(data) = self.data.take() {
            self.decompressed = decompress_to_vec(&data).ok()?;
        }
        let batch = Batch::try_from(self.decompressed.as_ref()).ok()?;
        Some(batch)
    }
}

impl From<&[u8]> for BatchReader {
    fn from(data: &[u8]) -> Self {
        Self {
            data: Some(data.to_vec()),
            decompressed: Vec::new(),
        }
    }
}
