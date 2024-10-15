//! This module contains the [ChannelAssembler] stage.

use super::{ChannelReaderProvider, NextFrameProvider};
use crate::{
    pipeline::{OriginAdvancer, PipelineResult, ResettableStage},
    prelude::{OriginProvider, PipelineError},
};
use alloc::{boxed::Box, sync::Arc};
use alloy_primitives::Bytes;
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::{RollupConfig, SystemConfig};
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
    P: NextFrameProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// The rollup configuration.
    cfg: Arc<RollupConfig>,
    /// The previous stage of the derivation pipeline.
    prev: P,
    /// The current [Channel] being assembled.
    channel: Option<Channel>,
}

impl<P> ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    /// Creates a new [ChannelAssembler] stage with the given configuration and previous stage.
    pub fn new(cfg: Arc<RollupConfig>, prev: P) -> Self {
        crate::set!(STAGE_RESETS, 0, &["channel-assembly"]);
        Self { cfg, prev, channel: None }
    }

    /// Consumes [self] and returns the previous stage.
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
                let timed_out = c.open_block_number() + self.cfg.channel_timeout(origin.timestamp) <
                    origin.number;
                if timed_out {
                    crate::observe!(
                        CHANNEL_TIMEOUTS,
                        (origin.number - c.open_block_number()) as f64
                    );
                }
                timed_out
            })
            .unwrap_or_default();

        Ok(is_timed_out)
    }
}

#[async_trait]
impl<P> ChannelReaderProvider for ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn next_data(&mut self) -> PipelineResult<Option<Bytes>> {
        if self.channel.is_some() && self.is_timed_out()? {
            self.channel = None;
        }

        // If the channel is already completed, and it hasn't been forwarded,
        // throw an error.
        if self.channel.as_ref().map(|c| c.is_ready()).unwrap_or_default() {
            return Err(PipelineError::ChannelAlreadyBuilt.crit());
        }

        let origin = self.origin().ok_or(PipelineError::MissingOrigin.crit())?;

        // Grab the next frame from the previous stage.
        let next_frame = self.prev.next_frame().await?;

        // Start a new channel if the frame number is 0.
        if next_frame.number == 0 {
            self.channel = Some(Channel::new(next_frame.id, origin));
        }

        // If the frame number is greater than 0, and the channel is not yet
        // started, return None.
        if next_frame.number > 0 && self.channel.is_none() {
            return Ok(None);
        }

        // Get a mutable reference to the stage's channel.
        let Some(channel) = self.channel.as_mut() else {
            return Err(PipelineError::ChannelNotFound.crit());
        };

        // Add the frame to the channel. If this fails, return None and discard the frame.
        if channel.add_frame(next_frame, origin).is_err() {
            return Ok(None);
        }

        // If the channel is ready, forward the channel to the next stage.
        if channel.is_ready() {
            let channel_bytes =
                channel.frame_data().ok_or(PipelineError::ChannelNotFound.crit())?;

            // Reset the channel and return the bytes.
            self.channel = None;
            return Ok(Some(channel_bytes));
        }

        Ok(None)
    }
}

#[async_trait]
impl<P> OriginAdvancer for ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        self.prev.advance_origin().await
    }
}

impl<P> OriginProvider for ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + ResettableStage + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P> ResettableStage for ChannelAssembler<P>
where
    P: NextFrameProvider + OriginAdvancer + OriginProvider + ResettableStage + Send + Debug,
{
    async fn reset(
        &mut self,
        block_info: BlockInfo,
        system_config: &SystemConfig,
    ) -> PipelineResult<()> {
        self.prev.reset(block_info, system_config).await?;
        self.channel = None;
        crate::inc!(STAGE_RESETS, &["channel-assembly"]);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    // TODO
}
