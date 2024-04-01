//! Module for working with span batch bits.

use crate::types::spans::{SpanBatchError, MAX_SPAN_BATCH_SIZE};
use alloc::vec;
use alloc::vec::Vec;
use alloy_rlp::Buf;
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
            b.advance(b.len());
            bits
        } else {
            let v = b[..buffer_len].to_vec();
            b.advance(buffer_len);
            v
        };
        if bits.iter().map(|n| n.count_ones()).sum::<u32>() as usize > bit_length {
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
            Some(if byte & (1 << (8 - bit_index)) != 0 {
                1
            } else {
                0
            })
        } else {
            // Return None if the index is out of bounds
            None
        }
    }

    /// Sets a bit in the [SpanBatchBits] bitlist.
    pub fn set_bit(&mut self, index: usize, value: bool) {
        let byte_index = index / 8;
        let bit_index = index % 8;

        // Ensure the vector is large enough to contain the bit at 'index'.
        // If not, resize the vector, filling with 0s.
        if byte_index >= self.0.len() {
            self.0.resize(byte_index + 1, 0);
        }

        // Retrieve the specific byte to modify
        let byte = &mut self.0[byte_index];

        if value {
            // Set the bit to 1
            *byte |= 1 << (8 - bit_index);
        } else {
            // Set the bit to 0
            *byte &= !(1 << (8 - bit_index));
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::{collection::vec, prelude::any, proptest};

    proptest! {
        #[test]
        fn test_encode_decode_roundtrip_span_bitlist(vec in vec(any::<u8>(), 0..5096)) {
            let bits = SpanBatchBits(vec);
            assert_eq!(SpanBatchBits::decode(&mut bits.as_ref(), bits.0.len() * 8).unwrap(), bits);
            let mut encoded = Vec::new();
            SpanBatchBits::encode(&mut encoded, bits.0.len() * 8, bits.as_ref()).unwrap();
            assert_eq!(encoded, bits.0);
        }
    }

    #[test]
    fn test_static_set_get_bits_span_bitlist() {
        let mut bits = SpanBatchBits::default();
        assert!(bits.0.is_empty());

        bits.set_bit(0, true);
        bits.set_bit(1, true);
        bits.set_bit(2, true);
        bits.set_bit(4, true);
        bits.set_bit(7, true);
        assert_eq!(bits.0.len(), 1);
        assert_eq!(bits.get_bit(0), Some(1));
        assert_eq!(bits.get_bit(1), Some(1));
        assert_eq!(bits.get_bit(2), Some(1));
        assert_eq!(bits.get_bit(3), Some(0));
        assert_eq!(bits.get_bit(4), Some(1));

        bits.set_bit(17, true);
        assert_eq!(bits.get_bit(17), Some(1));
        assert_eq!(bits.get_bit(32), None);
        assert_eq!(bits.0.len(), 3);
    }
}
