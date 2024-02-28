//! Module for working with span batch bits.

use crate::types::spans::{SpanBatchError, MAX_SPAN_BATCH_SIZE};
use alloc::vec;
use alloc::vec::Vec;
use alloy_primitives::U256;
use anyhow::Result;

/// Type for span batch bits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatchBits(pub Vec<u8>);

impl SpanBatchBits {
    /// Decodes a standard span-batch bitlist from a reader.
    /// The bitlist is encoded as big-endian integer, left-padded with zeroes to a multiple of 8 bits.
    /// The encoded bitlist cannot be longer than [MAX_SPAN_BATCH_SIZE].
    pub fn new(b: &mut &[u8], bit_length: usize) -> Result<Self, SpanBatchError> {
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
    pub fn encode(
        &self,
        w: &mut Vec<u8>,
        bit_length: usize,
        bits: U256,
    ) -> Result<(), SpanBatchError> {
        if bits.bit_len() > bit_length {
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
        let v = bits.to_be_bytes_vec();
        buf[buf_len - v.len()..].copy_from_slice(&v);
        w.extend_from_slice(&buf);
        Ok(())
    }
}
