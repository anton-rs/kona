//! This module contains the `ChannelBank` struct.

use crate::{
    errors::{StageError, StageResult},
    params::MAX_CHANNEL_BANK_SIZE,
    stages::ChannelReaderProvider,
    traits::{OriginAdvancer, OriginProvider, ResettableStage},
};
use alloc::{boxed::Box, collections::VecDeque, sync::Arc};
use alloy_primitives::{hex, Bytes};
use anyhow::anyhow;
use async_trait::async_trait;
use core::fmt::Debug;
use hashbrown::HashMap;
use kona_primitives::{BlockInfo, Channel, ChannelID, Frame, RollupConfig, SystemConfig};
use tracing::{trace, warn};

/// Provides frames for the [ChannelBank] stage.
#[async_trait]
pub trait ChannelBankProvider {
    /// Retrieves the next [Frame] from the [FrameQueue] stage.
    ///
    /// [FrameQueue]: crate::stages::FrameQueue
    async fn next_frame(&mut self) -> StageResult<Frame>;
}

/// [ChannelBank] is a stateful stage that does the following:
/// 1. Unmarshalls frames from L1 transaction data
/// 2. Applies those frames to a channel
/// 3. Attempts to read from the channel when it is ready
/// 4. Prunes channels (not frames) when the channel bank is too large.
///
/// Note: we prune before we ingest data.
/// As we switch between ingesting data & reading, the prune step occurs at an odd point
/// Specifically, the channel bank is not allowed to become too large between successive calls
/// to `IngestData`. This means that we can do an ingest and then do a read while becoming too
/// large. [ChannelBank] buffers channel frames, and emits full channel data
#[derive(Debug)]
pub struct ChannelBank<P>
where
    P: ChannelBankProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// The rollup configuration.
    cfg: Arc<RollupConfig>,
    /// Map of channels by ID.
    channels: HashMap<ChannelID, Channel>,
    /// Channels in FIFO order.
    channel_queue: VecDeque<ChannelID>,
    /// The previous stage of the derivation pipeline.
    prev: P,
}

impl<P> ChannelBank<P>
where
    P: ChannelBankProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// Create a new [ChannelBank] stage.
    pub fn new(cfg: Arc<RollupConfig>, prev: P) -> Self {
        crate::set!(STAGE_RESETS, 0, &["channel-bank"]);
        Self { cfg, channels: HashMap::new(), channel_queue: VecDeque::new(), prev }
    }

    /// Returns the size of the channel bank by accumulating over all channels.
    pub fn size(&self) -> usize {
        self.channels.iter().fold(0, |acc, (_, c)| acc + c.size())
    }

    /// Prunes the Channel bank, until it is below [MAX_CHANNEL_BANK_SIZE].
    /// Prunes from the high-priority channel since it failed to be read.
    pub fn prune(&mut self) -> StageResult<()> {
        let mut total_size = self.size();
        while total_size > MAX_CHANNEL_BANK_SIZE {
            let id = self.channel_queue.pop_front().ok_or(StageError::NoChannelsAvailable)?;
            let channel = self.channels.remove(&id).ok_or(StageError::ChannelNotFound)?;
            total_size -= channel.size();
        }
        Ok(())
    }

    /// Adds new L1 data to the channel bank. Should only be called after all data has been read.
    pub fn ingest_frame(&mut self, frame: Frame) -> StageResult<()> {
        let origin = self.origin().ok_or(StageError::MissingOrigin)?;

        // Get the channel for the frame, or create a new one if it doesn't exist.
        let current_channel = self.channels.entry(frame.id).or_insert_with(|| {
            let channel = Channel::new(frame.id, origin);
            self.channel_queue.push_back(frame.id);
            channel
        });

        // Check if the channel is not timed out. If it has, ignore the frame.
        if current_channel.open_block_number() + self.cfg.channel_timeout(origin.timestamp) <
            origin.number
        {
            warn!(
                target: "channel-bank",
                "Channel (ID: {}) timed out", hex::encode(frame.id)
            );
            return Ok(());
        }

        // Ingest the frame. If it fails, ignore the frame.
        let frame_id = frame.id;
        if current_channel.add_frame(frame, origin).is_err() {
            warn!(target: "channel-bank", "Failed to add frame to channel: {:?}", frame_id);
            return Ok(());
        }
        // Only increment the channel frames if the channel is current.
        if self.channel_queue.front().map_or(false, |id| *id == current_channel.id()) {
            crate::inc!(CURRENT_CHANNEL_FRAMES);
        }
        #[cfg(feature = "metrics")]
        {
            // For each channel, get the number of frames and record it in the CHANNEL_FRAME_COUNT
            // histogram metric.
            for (_, channel) in &self.channels {
                crate::observe!(CHANNEL_FRAME_COUNT, channel.len() as f64);
            }
        }

        self.prune()
    }

    /// Read the raw data of the first channel, if it's timed-out or closed.
    ///
    /// Returns an error if there is nothing new to read.
    pub fn read(&mut self) -> StageResult<Option<Bytes>> {
        // Bail if there are no channels to read from.
        if self.channel_queue.is_empty() {
            trace!(target: "channel-bank", "No channels to read from");
            return Err(StageError::Eof);
        }

        // Return an `Ok(None)` if the first channel is timed out. There may be more timed
        // out channels at the head of the queue and we want to remove them all.
        let first = self.channel_queue[0];
        let channel = self.channels.get(&first).ok_or(StageError::ChannelNotFound)?;
        let origin = self.origin().ok_or(StageError::MissingOrigin)?;
        if channel.open_block_number() + self.cfg.channel_timeout(origin.timestamp) < origin.number
        {
            warn!(
                target: "channel-bank",
                "Channel (ID: {}) timed out", hex::encode(first)
            );
            crate::observe!(CHANNEL_TIMEOUTS, (origin.number - channel.open_block_number()) as f64);
            self.channels.remove(&first);
            self.channel_queue.pop_front();
            crate::set!(
                CURRENT_CHANNEL_FRAMES,
                self.channel_queue.front().map_or(0, |id| self
                    .channels
                    .get(id)
                    .map_or(0, |c| c.len())
                    as i64)
            );
            return Ok(None);
        }

        // At this point we have removed all timed out channels from the front of the
        // `channel_queue`. Pre-Canyon we simply check the first index.
        // Post-Canyon we read the entire channelQueue for the first ready channel.
        // If no channel is available, we return StageError::Eof.
        // Canyon is activated when the first L1 block whose time >= CanyonTime, not on the L2
        // timestamp.
        if !self.cfg.is_canyon_active(origin.timestamp) {
            return self.try_read_channel_at_index(0).map(Some);
        }

        let channel_data =
            (0..self.channel_queue.len()).find_map(|i| self.try_read_channel_at_index(i).ok());
        match channel_data {
            Some(data) => Ok(Some(data)),
            None => Err(StageError::Eof),
        }
    }

    /// Attempts to read the channel at the specified index. If the channel is not ready or timed
    /// out, it will return an error.
    /// If the channel read was successful, it will remove the channel from the channel queue.
    fn try_read_channel_at_index(&mut self, index: usize) -> StageResult<Bytes> {
        let channel_id = self.channel_queue[index];
        let channel = self.channels.get(&channel_id).ok_or(StageError::ChannelNotFound)?;
        let origin = self.origin().ok_or(StageError::MissingOrigin)?;

        let timed_out = channel.open_block_number() + self.cfg.channel_timeout(origin.timestamp) <
            origin.number;
        if timed_out || !channel.is_ready() {
            return Err(StageError::Eof);
        }

        let frame_data = channel.frame_data();
        self.channels.remove(&channel_id);
        self.channel_queue.remove(index);

        frame_data.map_err(StageError::Custom)
    }
}

#[async_trait]
impl<P> OriginAdvancer for ChannelBank<P>
where
    P: ChannelBankProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn advance_origin(&mut self) -> StageResult<()> {
        self.prev.advance_origin().await
    }
}

#[async_trait]
impl<P> ChannelReaderProvider for ChannelBank<P>
where
    P: ChannelBankProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn next_data(&mut self) -> StageResult<Option<Bytes>> {
        crate::timer!(START, STAGE_ADVANCE_RESPONSE_TIME, &["channel_bank"], timer);
        match self.read() {
            Err(StageError::Eof) => {
                // continue - we will attempt to load data into the channel bank
            }
            Err(e) => {
                crate::timer!(DISCARD, timer);
                return Err(anyhow!("Error fetching next data from channel bank: {:?}", e).into());
            }
            data => return data,
        };

        // Load the data into the channel bank
        let frame = match self.prev.next_frame().await {
            Ok(f) => f,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                return Err(e);
            }
        };
        let res = self.ingest_frame(frame);
        crate::timer!(DISCARD, timer);
        res?;
        Err(StageError::NotEnoughData)
    }
}

impl<P> OriginProvider for ChannelBank<P>
where
    P: ChannelBankProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P> ResettableStage for ChannelBank<P>
where
    P: ChannelBankProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn reset(
        &mut self,
        block_info: BlockInfo,
        system_config: &SystemConfig,
    ) -> StageResult<()> {
        self.prev.reset(block_info, system_config).await?;
        self.channels.clear();
        self.channel_queue = VecDeque::with_capacity(10);
        crate::inc!(STAGE_RESETS, &["channel-bank"]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stages::{
        frame_queue::tests::new_test_frames,
        test_utils::{CollectingLayer, MockChannelBankProvider, TraceStorage},
    };
    use alloc::vec;
    use kona_primitives::{BASE_MAINNET_CONFIG, OP_MAINNET_CONFIG};
    use tracing::Level;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    #[test]
    fn test_ingest_empty_origin() {
        let mut mock = MockChannelBankProvider::new(vec![]);
        mock.block_info = None;
        let cfg = Arc::new(RollupConfig::default());
        let mut channel_bank = ChannelBank::new(cfg, mock);
        let frame = Frame::default();
        let err = channel_bank.ingest_frame(frame).unwrap_err();
        assert_eq!(err, StageError::MissingOrigin);
    }

    #[test]
    fn test_ingest_invalid_frame() {
        let trace_store: TraceStorage = Default::default();
        let layer = CollectingLayer::new(trace_store.clone());
        tracing_subscriber::Registry::default().with(layer).init();

        let mock = MockChannelBankProvider::new(vec![]);
        let mut channel_bank = ChannelBank::new(Arc::new(RollupConfig::default()), mock);
        let frame = Frame { id: [0xFF; 16], ..Default::default() };
        assert_eq!(channel_bank.size(), 0);
        assert!(channel_bank.channels.is_empty());
        assert_eq!(trace_store.lock().iter().filter(|(l, _)| matches!(l, &Level::WARN)).count(), 0);
        assert_eq!(channel_bank.ingest_frame(frame.clone()), Ok(()));
        assert_eq!(channel_bank.size(), kona_primitives::frame::FRAME_OVERHEAD);
        assert_eq!(channel_bank.channels.len(), 1);
        // This should fail since the frame is already ingested.
        assert_eq!(channel_bank.ingest_frame(frame), Ok(()));
        assert_eq!(channel_bank.size(), kona_primitives::frame::FRAME_OVERHEAD);
        assert_eq!(channel_bank.channels.len(), 1);
        assert_eq!(trace_store.lock().iter().filter(|(l, _)| matches!(l, &Level::WARN)).count(), 1);
    }

    #[test]
    fn test_ingest_and_prune_channel_bank() {
        use alloc::vec::Vec;
        let mut frames: Vec<Frame> = new_test_frames(100000);
        // let data = frames.iter().map(|f| Ok(f)).collect::<Vec<StageResult<Frame>>>();
        let mock = MockChannelBankProvider::new(vec![]);
        let cfg = Arc::new(RollupConfig::default());
        let mut channel_bank = ChannelBank::new(cfg, mock);
        // Ingest frames until the channel bank is full and it stops increasing in size
        let mut current_size = 0;
        let next_frame = frames.pop().unwrap();
        channel_bank.ingest_frame(next_frame).unwrap();
        while channel_bank.size() > current_size {
            current_size = channel_bank.size();
            let next_frame = frames.pop().unwrap();
            channel_bank.ingest_frame(next_frame).unwrap();
            assert!(channel_bank.size() <= MAX_CHANNEL_BANK_SIZE);
        }
        // There should be a bunch of frames leftover
        assert!(!frames.is_empty());
        // If we ingest one more frame, the channel bank should prune
        // and the size should be the same
        let next_frame = frames.pop().unwrap();
        channel_bank.ingest_frame(next_frame).unwrap();
        assert_eq!(channel_bank.size(), current_size);
    }

    #[tokio::test]
    async fn test_read_empty_channel_bank() {
        let frames = new_test_frames(1);
        let mock = MockChannelBankProvider::new(vec![Ok(frames[0].clone())]);
        let cfg = Arc::new(RollupConfig::default());
        let mut channel_bank = ChannelBank::new(cfg, mock);
        let err = channel_bank.read().unwrap_err();
        assert_eq!(err, StageError::Eof);
        let err = channel_bank.next_data().await.unwrap_err();
        assert_eq!(err, StageError::NotEnoughData);
    }

    #[tokio::test]
    async fn test_channel_timeout() {
        let trace_store: TraceStorage = Default::default();
        let layer = CollectingLayer::new(trace_store.clone());
        tracing_subscriber::Registry::default().with(layer).init();

        const ROLLUP_CONFIGS: [RollupConfig; 2] = [OP_MAINNET_CONFIG, BASE_MAINNET_CONFIG];

        for cfg in ROLLUP_CONFIGS {
            let frames = new_test_frames(2);
            let mock = MockChannelBankProvider::new(frames.into_iter().map(Ok).collect::<Vec<_>>());
            let cfg = Arc::new(cfg);
            let mut channel_bank = ChannelBank::new(cfg.clone(), mock);

            // Ingest first frame
            let err = channel_bank.next_data().await.unwrap_err();
            assert_eq!(err, StageError::NotEnoughData);

            for _ in 0..cfg.channel_timeout + 1 {
                channel_bank.advance_origin().await.unwrap();
            }

            // There should be an in-progress channel.
            assert_eq!(channel_bank.channels.len(), 1);
            assert_eq!(channel_bank.channel_queue.len(), 1);

            // Should be `Ok(())`, channel timed out.
            channel_bank.next_data().await.unwrap();

            // The channel should have been pruned.
            assert_eq!(channel_bank.channels.len(), 0);
            assert_eq!(channel_bank.channel_queue.len(), 0);

            // Ensure the channel was successfully timed out.
            let (_, warning_trace) = trace_store
                .lock()
                .iter()
                .find(|(l, _)| matches!(l, &Level::WARN))
                .cloned()
                .unwrap();
            assert!(warning_trace.contains("timed out"));
        }
    }
}
