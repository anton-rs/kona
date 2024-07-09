//! This module contains the `ChannelReader` struct.

use crate::{
    stages::{decompress_brotli, BatchQueueProvider},
    traits::{OriginAdvancer, OriginProvider, PreviousStage, ResettableStage},
    types::{Batch, BlockInfo, RollupConfig, StageError, StageResult, SystemConfig},
};

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_primitives::Bytes;
use alloy_rlp::Decodable;
use async_trait::async_trait;
use core::fmt::Debug;
use miniz_oxide::inflate::decompress_to_vec_zlib;
use tracing::{debug, error, warn};

/// ZLIB Deflate Compression Method.
pub(crate) const ZLIB_DEFLATE_COMPRESSION_METHOD: u8 = 8;

/// ZLIB Reserved Compression Info.
pub(crate) const ZLIB_RESERVED_COMPRESSION_METHOD: u8 = 15;

/// Brotili Compression Channel Version.
pub(crate) const CHANNEL_VERSION_BROTLI: u8 = 1;

/// The [ChannelReader] provider trait.
#[async_trait]
pub trait ChannelReaderProvider {
    /// Pulls the next piece of data from the channel bank. Note that it attempts to pull data out
    /// of the channel bank prior to loading data in (unlike most other stages). This is to
    /// ensure maintain consistency around channel bank pruning which depends upon the order
    /// of operations.
    async fn next_data(&mut self) -> StageResult<Option<Bytes>>;
}

/// [ChannelReader] is a stateful stage that does the following:
#[derive(Debug)]
pub struct ChannelReader<P>
where
    P: ChannelReaderProvider + PreviousStage + Debug,
{
    /// The previous stage of the derivation pipeline.
    prev: P,
    /// The batch reader.
    next_batch: Option<BatchReader>,
    /// The rollup coonfiguration.
    cfg: Arc<RollupConfig>,
}

impl<P> ChannelReader<P>
where
    P: ChannelReaderProvider + PreviousStage + Debug,
{
    /// Create a new [ChannelReader] stage.
    pub fn new(prev: P, cfg: Arc<RollupConfig>) -> Self {
        Self { prev, next_batch: None, cfg: cfg.clone() }
    }

    /// Creates the batch reader from available channel data.
    async fn set_batch_reader(&mut self) -> StageResult<()> {
        if self.next_batch.is_none() {
            let channel = self.prev.next_data().await?.ok_or(StageError::NoChannel)?;
            self.next_batch = Some(BatchReader::from(&channel[..]));
        }
        Ok(())
    }

    /// Forces the read to continue with the next channel, resetting any
    /// decoding / decompression state to a fresh start.
    pub fn next_channel(&mut self) {
        self.next_batch = None;
    }
}

#[async_trait]
impl<P> OriginAdvancer for ChannelReader<P>
where
    P: ChannelReaderProvider + PreviousStage + Send + Debug,
{
    async fn advance_origin(&mut self) -> StageResult<()> {
        self.prev.advance_origin().await
    }
}

#[async_trait]
impl<P> BatchQueueProvider for ChannelReader<P>
where
    P: ChannelReaderProvider + PreviousStage + Send + Debug,
{
    async fn next_batch(&mut self) -> StageResult<Batch> {
        crate::timer!(START, STAGE_ADVANCE_RESPONSE_TIME, &["channel_reader"], timer);
        if let Err(e) = self.set_batch_reader().await {
            debug!(target: "channel-reader", "Failed to set batch reader: {:?}", e);
            self.next_channel();
            crate::timer!(DISCARD, timer);
            return Err(e);
        }
        match self
            .next_batch
            .as_mut()
            .expect("Cannot be None")
            .next_batch(self.cfg.as_ref())
            .ok_or(StageError::NotEnoughData)
        {
            Ok(batch) => Ok(batch),
            Err(e) => {
                self.next_channel();
                crate::timer!(DISCARD, timer);
                Err(e)
            }
        }
    }
}

impl<P> OriginProvider for ChannelReader<P>
where
    P: ChannelReaderProvider + PreviousStage + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P> ResettableStage for ChannelReader<P>
where
    P: ChannelReaderProvider + PreviousStage + Debug + Send,
{
    async fn reset(&mut self, base: BlockInfo, cfg: &SystemConfig) -> StageResult<()> {
        self.prev.reset(base, cfg).await?;
        self.next_channel();
        Ok(())
    }
}

impl<P> PreviousStage for ChannelReader<P>
where
    P: ChannelReaderProvider + PreviousStage + Send + Debug,
{
    fn previous(&self) -> Option<Box<&dyn PreviousStage>> {
        Some(Box::new(&self.prev))
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
    /// The current cursor in the `decompressed` data.
    cursor: usize,
}

impl BatchReader {
    /// Pulls out the next batch from the reader.
    pub(crate) fn next_batch(&mut self, cfg: &RollupConfig) -> Option<Batch> {
        // If the data is not already decompressed, decompress it.
        let mut brotli_used = false;

        #[cfg(feature = "metrics")]
        let mut raw_len = 0;

        if let Some(data) = self.data.take() {
            // Peek at the data to determine the compression type.
            if data.is_empty() {
                warn!(target: "batch-reader", "Data is too short to determine compression type, skipping batch");
                return None;
            }

            #[cfg(feature = "metrics")]
            {
                raw_len = data.len();
            }

            let compression_type = data[0];
            if (compression_type & 0x0F) == ZLIB_DEFLATE_COMPRESSION_METHOD ||
                (compression_type & 0x0F) == ZLIB_RESERVED_COMPRESSION_METHOD
            {
                self.decompressed = decompress_to_vec_zlib(&data).ok()?;
            } else if compression_type == CHANNEL_VERSION_BROTLI {
                brotli_used = true;
                self.decompressed = decompress_brotli(&data[1..]).ok()?;
            } else {
                error!(target: "batch-reader", "Unsupported compression type: {:x}, skipping batch", compression_type);
                crate::inc!(BATCH_READER_ERRORS, &["unsupported_compression_type"]);
                return None;
            }
        }

        // Decompress and RLP decode the batch data, before finally decoding the batch itself.
        let decompressed_reader = &mut self.decompressed.as_slice()[self.cursor..].as_ref();
        let bytes = Bytes::decode(decompressed_reader).ok()?;
        crate::set!(BATCH_COMPRESSION_RATIO, (raw_len as i64) * 100 / bytes.len() as i64);
        let batch = Batch::decode(&mut bytes.as_ref(), cfg).unwrap();

        // Confirm that brotli decompression was performed *after* the Fjord hardfork.
        if brotli_used && !cfg.is_fjord_active(batch.timestamp()) {
            warn!(target: "batch-reader", "Brotli compression used before Fjord hardfork, skipping batch");
            crate::inc!(BATCH_READER_ERRORS, &["brotli_used_before_fjord"]);
            return None;
        }

        // Advance the cursor on the reader.
        self.cursor = self.decompressed.len() - decompressed_reader.len();

        Some(batch)
    }
}

impl<T: Into<Vec<u8>>> From<T> for BatchReader {
    fn from(data: T) -> Self {
        Self { data: Some(data.into()), decompressed: Vec::new(), cursor: 0 }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::stages::test_utils::MockChannelReaderProvider;
    use alloc::vec;

    fn new_compressed_batch_data() -> Bytes {
        let file_contents =
            alloc::string::String::from_utf8_lossy(include_bytes!("../../testdata/batch.hex"));
        let file_contents = &(&*file_contents)[..file_contents.len() - 1];
        let data = alloy_primitives::hex::decode(file_contents).unwrap();
        data.into()
    }

    #[tokio::test]
    async fn test_next_batch_batch_reader_set_fails() {
        let mock = MockChannelReaderProvider::new(vec![Err(StageError::Eof)]);
        let mut reader = ChannelReader::new(mock, Arc::new(RollupConfig::default()));
        assert_eq!(reader.next_batch().await, Err(StageError::Eof));
        assert!(reader.next_batch.is_none());
    }

    #[tokio::test]
    async fn test_next_batch_batch_reader_no_data() {
        let mock = MockChannelReaderProvider::new(vec![Ok(None)]);
        let mut reader = ChannelReader::new(mock, Arc::new(RollupConfig::default()));
        assert_eq!(reader.next_batch().await, Err(StageError::NoChannel));
        assert!(reader.next_batch.is_none());
    }

    #[tokio::test]
    async fn test_next_batch_batch_reader_not_enough_data() {
        let mut first = new_compressed_batch_data();
        let second = first.split_to(first.len() / 2);
        let mock = MockChannelReaderProvider::new(vec![Ok(Some(first)), Ok(Some(second))]);
        let mut reader = ChannelReader::new(mock, Arc::new(RollupConfig::default()));
        assert_eq!(reader.next_batch().await, Err(StageError::NotEnoughData));
        assert!(reader.next_batch.is_none());
    }

    #[tokio::test]
    async fn test_next_batch_succeeds() {
        let raw = new_compressed_batch_data();
        let mock = MockChannelReaderProvider::new(vec![Ok(Some(raw))]);
        let mut reader = ChannelReader::new(mock, Arc::new(RollupConfig::default()));
        let res = reader.next_batch().await.unwrap();
        matches!(res, Batch::Span(_));
        assert!(reader.next_batch.is_some());
    }

    #[test]
    fn test_batch_reader() {
        let raw = new_compressed_batch_data();
        let decompressed_len = decompress_to_vec_zlib(&raw).unwrap().len();
        let mut reader = BatchReader::from(raw);
        reader.next_batch(&RollupConfig::default()).unwrap();
        assert_eq!(reader.cursor, decompressed_len);
    }
}
