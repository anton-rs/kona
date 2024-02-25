//! This module contains the [Channel] struct.

use crate::{
    params::ChannelID,
    types::{BlockInfo, Frame},
};
use anyhow::{bail, Result};
use hashbrown::HashMap;

/// A Channel is a set of batches that are split into at least one, but possibly multiple frames.
/// Frames are allowed to be ingested out of order.
/// Each frame is ingested one by one. Once a frame with `closed` is added to the channel, the
/// channel may mark itself as ready for reading once all intervening frames have been added
#[derive(Debug, Clone, Default)]
pub struct Channel {
    /// The unique identifier for this channel
    id: ChannelID,
    /// The block that the channel is currently open at
    open_block: BlockInfo,
    /// Estimated memory size, used to drop the channel if we have too much data
    estimated_size: usize,
    /// True if the last frame has been buffered
    closed: bool,
    /// The highest frame number that has been ingested
    highest_frame_number: u16,
    /// The frame number of the frame where `is_last` is true
    /// No other frame number may be higher than this
    last_frame_number: u16,
    /// Store a map of frame number to frame for constant time ordering
    inputs: HashMap<u16, Frame>,
    /// The highest L1 inclusion block that a frame was included in
    highest_l1_inclusion_block: BlockInfo,
}

impl Channel {
    /// Create a new [Channel] with the given [ChannelID] and [BlockInfo].
    pub fn new(id: ChannelID, open_block: BlockInfo) -> Self {
        Self {
            id,
            open_block,
            inputs: HashMap::new(),
            ..Default::default()
        }
    }

    /// Add a frame to the channel.
    ///
    /// ## Takes
    /// - `frame`: The frame to add to the channel
    /// - `l1_inclusion_block`: The block that the frame was included in
    ///
    /// ## Returns
    /// - `Ok(()):` If the frame was successfully buffered
    /// - `Err(_):` If the frame was invalid
    pub fn add_frame(&mut self, frame: Frame, l1_inclusion_block: BlockInfo) -> Result<()> {
        // Ensure that the frame ID is equal to the channel ID.
        if frame.id != self.id {
            bail!("Frame ID does not match channel ID");
        }
        if frame.is_last && self.closed {
            bail!(
                "Cannot add ending frame to a closed channel. Channel ID: {:?}",
                self.id
            );
        }
        if !self.inputs.contains_key(&frame.number) {
            bail!(
                "Frame number already exists in channel. Channel ID: {:?}",
                self.id
            );
        }
        if self.closed && frame.number >= self.last_frame_number {
            bail!(
                "frame number {} is greater than or equal to end frame number {}",
                frame.number,
                self.last_frame_number
            );
        }

        // Guaranteed to succeed at this point. Update the channel state.
        if frame.is_last {
            self.last_frame_number = frame.number;
            self.closed = true;

            // Prune frames with a higher number than the last frame number when we receive a closing frame.
            if self.last_frame_number < self.highest_frame_number {
                self.inputs.retain(|id, frame| {
                    self.estimated_size -= frame.size();
                    *id < self.last_frame_number
                });
                self.highest_frame_number = self.last_frame_number;
            }
        }

        // Update the highest frame number.
        if frame.number > self.highest_frame_number {
            self.highest_frame_number = frame.number;
        }

        if self.highest_l1_inclusion_block.number < l1_inclusion_block.number {
            self.highest_l1_inclusion_block = l1_inclusion_block;
        }

        self.estimated_size += frame.size();
        self.inputs.insert(frame.number, frame);
        Ok(())
    }

    /// Returns the block number of the L1 block that contained the first [Frame] in this channel.
    pub fn open_block_number(&self) -> u64 {
        self.open_block.number
    }

    /// Returns the estimated size of the channel including [Frame] overhead.
    pub fn size(&self) -> usize {
        self.estimated_size
    }

    /// Returns `true` if the channel is ready to be read.
    pub fn is_ready(&self) -> bool {
        // Must have buffered the last frame before the channel is ready.
        if !self.closed {
            return false;
        }

        // Must have the possibility of contiguous frames.
        if self.inputs.len() != (self.last_frame_number + 1) as usize {
            return false;
        }

        // Check for contiguous frames.
        for i in 0..=self.last_frame_number {
            if !self.inputs.contains_key(&i) {
                return false;
            }
        }

        true
    }
}
