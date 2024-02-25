//! This module contains the [Frame] type used within the derivation pipeline.

use crate::params::{ChannelID, DERIVATION_VERSION_0};
use alloc::vec::Vec;
use anyhow::{anyhow, bail, Result};

/// Frames cannot be larger than 1MB.
/// Data transactions that carry frames are generally not larger than 128 KB due to L1 network conditions,
/// but we leave space to grow larger anyway (gas limit allows for more data).
const MAX_FRAME_LEN: usize = 1000;

/// A channel frame is a segment of a channel's data.
///
/// *Encoding*
/// frame = `channel_id ++ frame_number ++ frame_data_length ++ frame_data ++ is_last`
/// * channel_id        = bytes16
/// * frame_number      = uint16
/// * frame_data_length = uint32
/// * frame_data        = bytes
/// * is_last           = bool
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// The unique idetifier for the frame.
    pub id: ChannelID,
    /// The number of the frame.
    pub number: u16,
    /// The data within the frame.
    pub data: Vec<u8>,
    /// Whether or not the frame is the last in the sequence.
    pub is_last: bool,
}

impl Frame {
    /// Encode the frame into a byte vector.
    pub fn encode(&self) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(16 + 2 + 4 + self.data.len() + 1);
        encoded.extend_from_slice(&self.id);
        encoded.extend_from_slice(&self.number.to_be_bytes());
        encoded.extend_from_slice(&(self.data.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&self.data);
        encoded.push(self.is_last as u8);
        encoded
    }

    /// Decode a frame from a byte vector.
    pub fn decode(encoded: &[u8]) -> Result<(usize, Self)> {
        const BASE_FRAME_LEN: usize = 16 + 2 + 4 + 1;

        if encoded.len() < BASE_FRAME_LEN {
            bail!("Frame too short to decode");
        }

        let id = encoded[..16]
            .try_into()
            .map_err(|e| anyhow!("Error: {e}"))?;
        let number = u16::from_be_bytes(
            encoded[16..18]
                .try_into()
                .map_err(|e| anyhow!("Error: {e}"))?,
        );
        let data_len = u32::from_be_bytes(
            encoded[18..22]
                .try_into()
                .map_err(|e| anyhow!("Error: {e}"))?,
        ) as usize;
        let data = encoded[22..22 + data_len].to_vec();
        let is_last = encoded[22 + data_len] != 0;
        Ok((
            BASE_FRAME_LEN + data_len,
            Self {
                id,
                number,
                data,
                is_last,
            },
        ))
    }

    /// ParseFrames parse the on chain serialization of frame(s) in an L1 transaction. Currently only version 0 of the
    /// serialization format is supported. All frames must be parsed without error and there must not be any left over
    /// data and there must be at least one frame.
    ///
    /// Frames are stored in L1 transactions with the following format:
    /// * `data = DerivationVersion0 ++ Frame(s)`
    /// Where there is one or more frames concatenated together.
    fn parse_frames(encoded: &[u8]) -> Result<Vec<Self>> {
        if encoded.is_empty() {
            bail!("No frames to parse");
        }
        if encoded[0] != DERIVATION_VERSION_0 {
            bail!("Unsupported derivation version");
        }

        let data = &encoded[1..];
        let mut frames = Vec::new();
        let mut offset = 0;
        while offset < data.len() {
            let (frame_length, frame) = Self::decode(&data[offset..])?;
            frames.push(frame);
            offset += frame_length;
        }

        if offset != data.len() {
            bail!("Frame data length mismatch");
        }
        if frames.is_empty() {
            bail!("No frames decoded");
        }

        Ok(frames)
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;

    #[test]
    fn test_encode_frame_roundtrip() {
        let frame = Frame {
            id: [1; 16],
            number: 0,
            data: std::vec![],
            is_last: true,
        };

        let (_, frame_decoded) = Frame::decode(&frame.encode()).unwrap();
        assert_eq!(frame, frame_decoded);
    }

    #[test]
    fn test_decode_many() {
        let frame = Frame {
            id: [1; 16],
            number: 0,
            data: std::vec![],
            is_last: true,
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[DERIVATION_VERSION_0]);
        (0..5).for_each(|_| {
            bytes.extend_from_slice(&frame.encode());
        });

        let frames = Frame::parse_frames(bytes.as_slice()).unwrap();
        assert_eq!(frames.len(), 5);
        (0..5).for_each(|i| {
            assert_eq!(frames[i], frame);
        });
    }
}
