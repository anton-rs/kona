//! This module contains the [ChannelAssembler] stage.

use super::{ChannelReaderProvider, NextFrameProvider};
use crate::{
    pipeline::{OriginAdvancer, PipelineResult, Signal, SignalReceiver},
    prelude::{OriginProvider, PipelineError},
};
use alloc::{boxed::Box, sync::Arc};
use alloy_primitives::Bytes;
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::{BlockInfo, Channel};

/// The [ChannelAssembler] stage is responsible for assembling the [Frame]s from the [FrameQueue]
/// stage into a raw compressed [Channel].
///
/// [Frame]: op_alloy_protocol::Frame
/// [FrameQueue]: crate::stages::FrameQueue
/// [Channel]: op_alloy_protocol::Channel
#[derive(Debug)]
pub struct ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug,
{
    /// The rollup configuration.
    pub(crate) cfg: Arc<RollupConfig>,
    /// The previous stage of the derivation pipeline.
    pub(crate) prev: P,
    /// The current [Channel] being assembled.
    pub(crate) channel: Option<Channel>,
}

impl<P> ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug,
{
    /// Creates a new [ChannelAssembler] stage with the given configuration and previous stage.
    pub fn new(cfg: Arc<RollupConfig>, prev: P) -> Self {
        crate::set!(STAGE_RESETS, 0, &["channel-assembly"]);
        Self { cfg, prev, channel: None }
    }

    /// Consumes [Self] and returns the previous stage.
    pub fn into_prev(self) -> P {
        self.prev
    }

    /// Returns whether or not the channel currently being assembled has timed out.
    pub fn is_timed_out(&self) -> PipelineResult<bool> {
        let origin = self.origin().ok_or(PipelineError::MissingOrigin.crit())?;
        let is_timed_out = self
            .channel
            .as_ref()
            .map(|c| {
                c.open_block_number() + self.cfg.channel_timeout(origin.timestamp) < origin.number
            })
            .unwrap_or_default();

        Ok(is_timed_out)
    }
}

#[async_trait]
impl<P> ChannelReaderProvider for ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + SignalReceiver + Send + Debug,
{
    async fn next_data(&mut self) -> PipelineResult<Option<Bytes>> {
        let origin = self.origin().ok_or(PipelineError::MissingOrigin.crit())?;

        // Time out the channel if it has timed out.
        if self.channel.is_some() && self.is_timed_out()? {
            #[cfg(feature = "metrics")]
            {
                let open_block_number =
                    self.channel.as_ref().map(|c| c.open_block_number()).unwrap_or_default();
                crate::observe!(CHANNEL_TIMEOUTS, (origin.number - open_block_number) as f64);
            }
            self.channel = None;
        }

        // Grab the next frame from the previous stage.
        let next_frame = self.prev.next_frame().await?;

        // Start a new channel if the frame number is 0.
        if next_frame.number == 0 {
            self.channel = Some(Channel::new(next_frame.id, origin));
        }

        if let Some(channel) = self.channel.as_mut() {
            // Add the frame to the channel. If this fails, return NotEnoughData and discard the
            // frame.
            if channel.add_frame(next_frame, origin).is_err() {
                return Err(PipelineError::NotEnoughData.temp());
            }

            // If the channel is ready, forward the channel to the next stage.
            if channel.is_ready() {
                let channel_bytes =
                    channel.frame_data().ok_or(PipelineError::ChannelNotFound.crit())?;

                // Reset the channel and return the compressed bytes.
                self.channel = None;
                return Ok(Some(channel_bytes));
            }
        }

        Err(PipelineError::NotEnoughData.temp())
    }
}

#[async_trait]
impl<P> OriginAdvancer for ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + SignalReceiver + Send + Debug,
{
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        self.prev.advance_origin().await
    }
}

impl<P> OriginProvider for ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P> SignalReceiver for ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + SignalReceiver + Send + Debug,
{
    async fn signal(&mut self, signal: Signal) -> PipelineResult<()> {
        self.prev.signal(signal).await?;
        self.channel = None;
        crate::inc!(STAGE_RESETS, &["channel-assembly"]);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::ChannelAssembler;
    use crate::{
        prelude::PipelineError,
        stages::{frame_queue::tests::new_test_frames, ChannelReaderProvider},
        test_utils::TestNextFrameProvider,
    };
    use alloc::sync::Arc;
    use op_alloy_genesis::RollupConfig;
    use op_alloy_protocol::BlockInfo;

    #[tokio::test]
    async fn test_assembler_channel_timeout() {
        let frames = new_test_frames(2);
        let mock = TestNextFrameProvider::new(frames.into_iter().rev().map(Ok).collect());
        let cfg = Arc::new(RollupConfig::default());
        let mut assembler = ChannelAssembler::new(cfg, mock);

        // Set the origin to default block info @ block # 0.
        assembler.prev.block_info = Some(BlockInfo::default());

        // Read in the first frame. Since the frame isn't the last, the assembler
        // should return None.
        assert!(assembler.channel.is_none());
        assert_eq!(assembler.next_data().await.unwrap_err(), PipelineError::NotEnoughData.temp());
        assert!(assembler.channel.is_some());

        // Push the origin forward past channel timeout.
        assembler.prev.block_info =
            Some(BlockInfo { number: assembler.cfg.channel_timeout(0) + 1, ..Default::default() });

        // Assert that the assembler has timed out the channel.
        assert!(assembler.is_timed_out().unwrap());
        assert_eq!(assembler.next_data().await.unwrap_err(), PipelineError::NotEnoughData.temp());
        assert!(assembler.channel.is_none());
    }

    #[tokio::test]
    async fn test_assembler_non_starting_frame() {
        let frames = new_test_frames(2);
        let mock = TestNextFrameProvider::new(frames.into_iter().map(Ok).collect());
        let cfg = Arc::new(RollupConfig::default());
        let mut assembler = ChannelAssembler::new(cfg, mock);

        // Send in the second frame first. This should result in no channel being created,
        // and the frame being discarded.
        assert!(assembler.channel.is_none());
        assert_eq!(assembler.next_data().await.unwrap_err(), PipelineError::NotEnoughData.temp());
        assert!(assembler.channel.is_none());
    }

    #[tokio::test]
    async fn test_assembler_already_built() {
        let frames = new_test_frames(2);
        let mock = TestNextFrameProvider::new(frames.clone().into_iter().rev().map(Ok).collect());
        let cfg = Arc::new(RollupConfig::default());
        let mut assembler = ChannelAssembler::new(cfg, mock);

        // Send in the first frame. This should result in a channel being created.
        assert!(assembler.channel.is_none());
        assert_eq!(assembler.next_data().await.unwrap_err(), PipelineError::NotEnoughData.temp());
        assert!(assembler.channel.is_some());

        // Send in a malformed second frame. This should result in an error in `add_frame`.
        assembler.prev.data.push(Ok(frames[1].clone()).map(|mut f| {
            f.id = Default::default();
            f
        }));
        assert_eq!(assembler.next_data().await.unwrap_err(), PipelineError::NotEnoughData.temp());
        assert!(assembler.channel.is_some());

        // Send in the second frame again. This should return the channel bytes.
        assert!(assembler.next_data().await.unwrap().is_some());
        assert!(assembler.channel.is_none());
    }
}
