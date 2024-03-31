//! Module for working with span batch bits.

use crate::types::spans::{SpanBatchError, MAX_SPAN_BATCH_SIZE};
use alloc::vec;
use alloc::vec::Vec;
use alloy_primitives::U256;
use anyhow::Result;

/// Type for span batch bits.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SpanBatchBits(pub Vec<u8>);

impl AsRef<Vec<u8>> for SpanBatchBits {
    fn as_ref(&self) -> &Vec<u8> {
        &self.0
    }
}

impl AsRef<[u8]> for SpanBatchBits {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<SpanBatchBits> for Vec<u8> {
    fn from(bits: SpanBatchBits) -> Vec<u8> {
        bits.0
    }
}

impl SpanBatchBits {
    /// Decodes a standard span-batch bitlist from a reader.
    /// The bitlist is encoded as big-endian integer, left-padded with zeroes to a multiple of 8 bits.
    /// The encoded bitlist cannot be longer than [MAX_SPAN_BATCH_SIZE].
    pub fn decode(b: &mut &[u8], bit_length: usize) -> Result<Self, SpanBatchError> {
        let buffer_len = bit_length / 8 + if bit_length % 8 != 0 { 1 } else { 0 };
        if buffer_len > MAX_SPAN_BATCH_SIZE {
            return Err(SpanBatchError::TooBigSpanBatchSize);
        }

        // TODO(refcell): This can definitely be optimized.
        let bits = if b.len() < buffer_len {
            let mut bits = vec![0; buffer_len];
            bits[..b.len()].copy_from_slice(b);
            bits
        } else {
            b[..buffer_len].to_vec()
        };
        let out = U256::try_from_be_slice(&bits).ok_or(SpanBatchError::InvalidBitSlice)?;
        if out.bit_len() > bit_length {
            return Err(SpanBatchError::BitfieldTooLong);
        }
        Ok(SpanBatchBits(bits.to_vec()))
    }

    /// Encodes a standard span-batch bitlist.
    /// The bitlist is encoded as big-endian integer, left-padded with zeroes to a multiple of 8 bits.
    /// The encoded bitlist cannot be longer than [MAX_SPAN_BATCH_SIZE].
    pub fn encode(w: &mut Vec<u8>, bit_length: usize, bits: &[u8]) -> Result<(), SpanBatchError> {
        if bits.len() * 8 > bit_length {
            return Err(SpanBatchError::BitfieldTooLong);
        }
        // Round up, ensure enough bytes when number of bits is not a multiple of 8.
        // Alternative of (L+7)/8 is not overflow-safe.
        let buf_len = bit_length / 8 + if bit_length % 8 != 0 { 1 } else { 0 };
        if buf_len > MAX_SPAN_BATCH_SIZE {
            return Err(SpanBatchError::TooBigSpanBatchSize);
        }
        // TODO(refcell): This can definitely be optimized.
        let mut buf = vec![0; buf_len];
        buf[buf_len - bits.len()..].copy_from_slice(bits);
        w.extend_from_slice(&buf);
        Ok(())
    }

    /// Get a bit from the [SpanBatchBits] bitlist.
    pub fn get_bit(&self, index: usize) -> Option<u8> {
        let byte_index = index / 8;
        let bit_index = index % 8;

        // Check if the byte index is within the bounds of the bitlist
        if byte_index < self.0.len() {
            // Retrieve the specific byte that contains the bit we're interested in
            let byte = self.0[byte_index];

            // Shift the bits of the byte to the right, based on the bit index, and
            // mask it with 1 to isolate the bit we're interested in.
            // If the result is not zero, the bit is set to 1, otherwise it's 0.
            Some(if byte & (1 << bit_index) != 0 { 1 } else { 0 })
        } else {
            // Return None if the index is out of bounds
            None
        }
    }
}
