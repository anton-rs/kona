//! This module contains the `ChannelReader` struct.

use crate::{
    errors::{PipelineError, PipelineResult},
    metrics::PipelineMetrics,
    stages::{decompress_brotli, BatchStreamProvider},
    traits::{ChannelReaderMetrics, OriginAdvancer, OriginProvider, Signal, SignalReceiver},
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_primitives::Bytes;
use alloy_rlp::Decodable;
use async_trait::async_trait;
use core::fmt::Debug;
use miniz_oxide::inflate::decompress_to_vec_zlib;
use op_alloy_genesis::{
    RollupConfig, MAX_RLP_BYTES_PER_CHANNEL_BEDROCK, MAX_RLP_BYTES_PER_CHANNEL_FJORD,
};
use op_alloy_protocol::{Batch, BlockInfo};
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
    async fn next_data(&mut self) -> PipelineResult<Option<Bytes>>;
}

/// [ChannelReader] is a stateful stage that reads [Batch]es from `Channel`s.
///
/// The [ChannelReader] pulls `Channel`s from the channel bank as raw data
/// and pipes it into a `BatchReader`. Since the raw data is compressed,
/// the `BatchReader` first decompresses the data using the first bytes as
/// a compression algorithm identifier.
///
/// Once the data is decompressed, it is decoded into a `Batch` and passed
/// to the next stage in the pipeline.
#[derive(Debug)]
pub struct ChannelReader<P>
where
    P: ChannelReaderProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug,
{
    /// The previous stage of the derivation pipeline.
    prev: P,
    /// The batch reader.
    next_batch: Option<BatchReader>,
    /// The rollup coonfiguration.
    cfg: Arc<RollupConfig>,
    /// Metrics collector.
    metrics: PipelineMetrics,
}

impl<P> ChannelReader<P>
where
    P: ChannelReaderProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug,
{
    /// Create a new [ChannelReader] stage.
    pub const fn new(prev: P, cfg: Arc<RollupConfig>, metrics: PipelineMetrics) -> Self {
        Self { prev, next_batch: None, cfg, metrics }
    }

    /// Creates the batch reader from available channel data.
    async fn set_batch_reader(&mut self) -> PipelineResult<()> {
        if self.next_batch.is_none() {
            let channel =
                self.prev.next_data().await?.ok_or(PipelineError::ChannelReaderEmpty.temp())?;

            let origin = self.prev.origin().ok_or(PipelineError::MissingOrigin.crit())?;
            let max_rlp_bytes_per_channel = if self.cfg.is_fjord_active(origin.timestamp) {
                MAX_RLP_BYTES_PER_CHANNEL_FJORD
            } else {
                MAX_RLP_BYTES_PER_CHANNEL_BEDROCK
            };

            self.next_batch =
                Some(BatchReader::new(&channel[..], max_rlp_bytes_per_channel as usize));
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
    P: ChannelReaderProvider + OriginAdvancer + OriginProvider + SignalReceiver + Send + Debug,
{
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        self.prev.advance_origin().await
    }
}

#[async_trait]
impl<P> BatchStreamProvider for ChannelReader<P>
where
    P: ChannelReaderProvider + OriginAdvancer + OriginProvider + SignalReceiver + Send + Debug,
{
    /// This method is called by the BatchStream if an invalid span batch is found.
    /// In the case of an invalid span batch, the associated channel must be flushed.
    ///
    /// See: <https://specs.optimism.io/protocol/holocene/derivation.html#span-batches>
    ///
    /// SAFETY: Only called post-holocene activation.
    fn flush(&mut self) {
        debug!(target: "channel-reader", "[POST-HOLOCENE] Flushing channel");
        self.metrics.record_channel_flushed();
        self.next_channel();
    }

    async fn next_batch(&mut self) -> PipelineResult<Batch> {
        if let Err(e) = self.set_batch_reader().await {
            debug!(target: "channel-reader", "Failed to set batch reader: {:?}", e);
            self.next_channel();
            return Err(e);
        }
        match self
            .next_batch
            .as_mut()
            .expect("Cannot be None")
            .next_batch(self.cfg.as_ref())
            .ok_or(PipelineError::NotEnoughData.temp())
        {
            Ok(batch) => {
                self.metrics.record_batch_read();
                Ok(batch)
            }
            Err(e) => {
                self.next_channel();
                Err(e)
            }
        }
    }
}

impl<P> OriginProvider for ChannelReader<P>
where
    P: ChannelReaderProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P> SignalReceiver for ChannelReader<P>
where
    P: ChannelReaderProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug + Send,
{
    async fn signal(&mut self, signal: Signal) -> PipelineResult<()> {
        match signal {
            Signal::FlushChannel => {
                // Drop the current in-progress channel.
                warn!(target: "channel-reader", "Flushed channel");
                self.next_batch = None;
            }
            s => {
                self.prev.signal(s).await?;
                self.next_channel();
            }
        }
        Ok(())
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
    /// The maximum RLP bytes per channel.
    max_rlp_bytes_per_channel: usize,
}

impl BatchReader {
    /// Creates a new [BatchReader] from the given data and max decompressed RLP bytes per channel.
    pub(crate) fn new<T>(data: T, max_rlp_bytes_per_channel: usize) -> Self
    where
        T: Into<Vec<u8>>,
    {
        Self {
            data: Some(data.into()),
            decompressed: Vec::new(),
            cursor: 0,
            max_rlp_bytes_per_channel,
        }
    }

    /// Pulls out the next batch from the reader.
    pub(crate) fn next_batch(&mut self, cfg: &RollupConfig) -> Option<Batch> {
        // If the data is not already decompressed, decompress it.
        let mut brotli_used = false;

        if let Some(data) = self.data.take() {
            // Peek at the data to determine the compression type.
            if data.is_empty() {
                warn!(target: "batch-reader", "Data is too short to determine compression type, skipping batch");
                return None;
            }

            let compression_type = data[0];
            if (compression_type & 0x0F) == ZLIB_DEFLATE_COMPRESSION_METHOD ||
                (compression_type & 0x0F) == ZLIB_RESERVED_COMPRESSION_METHOD
            {
                self.decompressed = decompress_to_vec_zlib(&data).ok()?;

                // Check the size of the decompressed channel RLP.
                if self.decompressed.len() > self.max_rlp_bytes_per_channel {
                    return None;
                }
            } else if compression_type == CHANNEL_VERSION_BROTLI {
                brotli_used = true;
                self.decompressed =
                    decompress_brotli(&data[1..], self.max_rlp_bytes_per_channel).ok()?;
            } else {
                error!(target: "batch-reader", "Unsupported compression type: {:x}, skipping batch", compression_type);
                return None;
            }
        }

        // Decompress and RLP decode the batch data, before finally decoding the batch itself.
        let decompressed_reader = &mut self.decompressed.as_slice()[self.cursor..].as_ref();
        let bytes = Bytes::decode(decompressed_reader).ok()?;
        let Ok(batch) = Batch::decode(&mut bytes.as_ref(), cfg) else {
            error!(target: "batch-reader", "Failed to decode batch, skipping batch");
            return None;
        };

        // Confirm that brotli decompression was performed *after* the Fjord hardfork.
        if brotli_used && !cfg.is_fjord_active(batch.timestamp()) {
            warn!(target: "batch-reader", "Brotli compression used before Fjord hardfork, skipping batch");
            return None;
        }

        // Advance the cursor on the reader.
        self.cursor = self.decompressed.len() - decompressed_reader.len();
        Some(batch)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        errors::PipelineErrorKind, test_utils::TestChannelReaderProvider, traits::ResetSignal,
    };
    use alloc::vec;

    fn new_compressed_batch_data() -> Bytes {
        let file_contents =
            alloc::string::String::from_utf8_lossy(include_bytes!("../../../testdata/batch.hex"));
        let file_contents = &(&*file_contents)[..file_contents.len() - 1];
        let data = alloy_primitives::hex::decode(file_contents).unwrap();
        data.into()
    }

    #[tokio::test]
    async fn test_flush_channel_reader() {
        let mock = TestChannelReaderProvider::new(vec![Ok(Some(new_compressed_batch_data()))]);
        let mut reader =
            ChannelReader::new(mock, Arc::new(RollupConfig::default()), PipelineMetrics::no_op());
        reader.next_batch = Some(BatchReader::new(
            new_compressed_batch_data(),
            MAX_RLP_BYTES_PER_CHANNEL_FJORD as usize,
        ));
        reader.signal(Signal::FlushChannel).await.unwrap();
        assert!(reader.next_batch.is_none());
    }

    #[tokio::test]
    async fn test_reset_channel_reader() {
        let mock = TestChannelReaderProvider::new(vec![Ok(None)]);
        let mut reader =
            ChannelReader::new(mock, Arc::new(RollupConfig::default()), PipelineMetrics::no_op());
        reader.next_batch = Some(BatchReader::new(
            vec![0x00, 0x01, 0x02],
            MAX_RLP_BYTES_PER_CHANNEL_FJORD as usize,
        ));
        assert!(!reader.prev.reset);
        reader.signal(ResetSignal::default().signal()).await.unwrap();
        assert!(reader.next_batch.is_none());
        assert!(reader.prev.reset);
    }

    #[tokio::test]
    async fn test_next_batch_batch_reader_set_fails() {
        let mock = TestChannelReaderProvider::new(vec![Err(PipelineError::Eof.temp())]);
        let mut reader =
            ChannelReader::new(mock, Arc::new(RollupConfig::default()), PipelineMetrics::no_op());
        assert_eq!(reader.next_batch().await, Err(PipelineError::Eof.temp()));
        assert!(reader.next_batch.is_none());
    }

    #[tokio::test]
    async fn test_next_batch_batch_reader_no_data() {
        let mock = TestChannelReaderProvider::new(vec![Ok(None)]);
        let mut reader =
            ChannelReader::new(mock, Arc::new(RollupConfig::default()), PipelineMetrics::no_op());
        assert!(matches!(
            reader.next_batch().await.unwrap_err(),
            PipelineErrorKind::Temporary(PipelineError::ChannelReaderEmpty)
        ));
        assert!(reader.next_batch.is_none());
    }

    #[tokio::test]
    async fn test_next_batch_batch_reader_not_enough_data() {
        let mut first = new_compressed_batch_data();
        let second = first.split_to(first.len() / 2);
        let mock = TestChannelReaderProvider::new(vec![Ok(Some(first)), Ok(Some(second))]);
        let mut reader =
            ChannelReader::new(mock, Arc::new(RollupConfig::default()), PipelineMetrics::no_op());
        assert_eq!(reader.next_batch().await, Err(PipelineError::NotEnoughData.temp()));
        assert!(reader.next_batch.is_none());
    }

    #[tokio::test]
    async fn test_next_batch_succeeds() {
        let raw = new_compressed_batch_data();
        let mock = TestChannelReaderProvider::new(vec![Ok(Some(raw))]);
        let mut reader =
            ChannelReader::new(mock, Arc::new(RollupConfig::default()), PipelineMetrics::no_op());
        let res = reader.next_batch().await.unwrap();
        matches!(res, Batch::Span(_));
        assert!(reader.next_batch.is_some());
    }

    #[test]
    fn test_batch_reader() {
        let raw = new_compressed_batch_data();
        let decompressed_len = decompress_to_vec_zlib(&raw).unwrap().len();
        let mut reader = BatchReader::new(raw, MAX_RLP_BYTES_PER_CHANNEL_BEDROCK as usize);
        reader.next_batch(&RollupConfig::default()).unwrap();
        assert_eq!(reader.cursor, decompressed_len);
    }

    #[test]
    fn test_batch_reader_fjord() {
        let raw = new_compressed_batch_data();
        let decompressed_len = decompress_to_vec_zlib(&raw).unwrap().len();
        let mut reader = BatchReader::new(raw, MAX_RLP_BYTES_PER_CHANNEL_FJORD as usize);
        reader.next_batch(&RollupConfig { fjord_time: Some(0), ..Default::default() }).unwrap();
        assert_eq!(reader.cursor, decompressed_len);
    }

    #[tokio::test]
    async fn test_flush_post_holocene() {
        let raw = new_compressed_batch_data();
        let config = Arc::new(RollupConfig { holocene_time: Some(0), ..RollupConfig::default() });
        let mock = TestChannelReaderProvider::new(vec![Ok(Some(raw))]);
        let mut reader = ChannelReader::new(mock, config, PipelineMetrics::no_op());
        let res = reader.next_batch().await.unwrap();
        matches!(res, Batch::Span(_));
        assert!(reader.next_batch.is_some());
        reader.flush();
        assert!(reader.next_batch.is_none());
    }
}
