//! Raw Span Batch Prefix

use crate::types::spans::{SpanBatchError, SpanDecodingError};
use alloc::vec::Vec;
use alloy_primitives::{FixedBytes, U64};
use alloy_rlp::{Decodable, Encodable};

/// Span Batch Prefix
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanBatchPrefix {
    /// Relative timestamp of the first block
    pub rel_timestamp: U64,
    /// L1 origin number
    pub l1_origin_num: U64,
    /// First 20 bytes of the first block's parent hash
    pub parent_check: FixedBytes<20>,
    /// First 20 bytes of the last block's L1 origin hash
    pub l1_origin_check: FixedBytes<20>,
}

impl Decodable for SpanBatchPrefix {
    fn decode(r: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let mut prefix = SpanBatchPrefix::default();
        prefix
            .decode_rel_timestamp(r)
            .map_err(|_| alloy_rlp::Error::Custom("Decoding relative timestamp failed"))?;
        prefix
            .decode_l1_origin_num(r)
            .map_err(|_| alloy_rlp::Error::Custom("Decoding L1 origin number failed"))?;
        prefix
            .decode_parent_check(r)
            .map_err(|_| alloy_rlp::Error::Custom("Decoding parent check failed"))?;
        prefix
            .decode_l1_origin_check(r)
            .map_err(|_| alloy_rlp::Error::Custom("Decoding L1 origin check failed"))?;
        Ok(prefix)
    }
}

impl Encodable for SpanBatchPrefix {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.rel_timestamp.encode(out);
        self.l1_origin_num.encode(out);
        self.parent_check.encode(out);
        self.l1_origin_check.encode(out);
    }
}

impl SpanBatchPrefix {
    /// Decodes the relative timestamp from a reader.
    pub fn decode_rel_timestamp(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let rel_timestamp = U64::decode(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::RelativeTimestamp))?;
        self.rel_timestamp = rel_timestamp;
        Ok(())
    }

    /// Decodes the L1 origin number from a reader.
    pub fn decode_l1_origin_num(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let l1_origin_num = U64::decode(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::L1OriginNumber))?;
        self.l1_origin_num = l1_origin_num;
        Ok(())
    }

    /// Decodes the parent check from a reader.
    pub fn decode_parent_check(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let parent_check = FixedBytes::<20>::decode(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::ParentCheck))?;
        self.parent_check = parent_check;
        Ok(())
    }

    /// Decodes the L1 origin check from a reader.
    pub fn decode_l1_origin_check(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let l1_origin_check = FixedBytes::decode(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::L1OriginCheck))?;
        self.l1_origin_check = l1_origin_check;
        Ok(())
    }

    /// Returns the length of the RLP encoding of the prefix.
    pub fn rlp_encoded_len(&self) -> usize {
        self.rel_timestamp.length()
            + self.l1_origin_num.length()
            + self.parent_check.length()
            + self.l1_origin_check.length()
    }

    /// Returns the rlp encoding of the prefix.
    pub fn rlp_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new(); // with_capacity(self.rlp_encoded_len());
        self.encode(&mut buf);
        buf
    }
}

#[cfg(test)]
mod test {
    use super::{SpanBatchError, SpanBatchPrefix, SpanDecodingError};
    use alloc::vec::Vec;
    use alloy_primitives::{FixedBytes, U64};
    use alloy_rlp::{Decodable, Encodable};

    #[test]
    fn test_span_batch_prefix_roundtrip_encoding() {
        let expected = SpanBatchPrefix {
            rel_timestamp: U64::from(1337),
            l1_origin_num: U64::from(42),
            parent_check: [0x42; 20].into(),
            l1_origin_check: [0x42; 20].into(),
        };
        let encoded = expected.rlp_bytes();
        let result = SpanBatchPrefix::decode(&mut &encoded[..]);
        assert_eq!(result, Ok(expected));
    }

    #[test]
    fn test_decode_rel_timestamp() {
        let expected = U64::from(1337);
        let mut buf = Vec::with_capacity(expected.length());
        expected.encode(&mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_rel_timestamp(&mut r).unwrap();
        assert_eq!(prefix.rel_timestamp, expected);
    }

    #[test]
    fn test_decode_l1_origin_num() {
        let expected = U64::from(1337);
        let mut buf = Vec::with_capacity(expected.length());
        expected.encode(&mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_l1_origin_num(&mut r).unwrap();
        assert_eq!(prefix.l1_origin_num, expected);
    }

    #[test]
    fn test_decode_parent_check() {
        let expected = FixedBytes::<20>::from([0x42; 20]);
        let mut buf = Vec::with_capacity(expected.length());
        expected.encode(&mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_parent_check(&mut r).unwrap();
        assert_eq!(prefix.parent_check, expected);
    }

    #[test]
    fn test_decode_parent_check_short() {
        let r = &[0x42; 19];
        let mut r = &r[..];
        let mut prefix = SpanBatchPrefix::default();
        let result = prefix.decode_parent_check(&mut r);
        assert_eq!(
            result,
            Err(SpanBatchError::Decoding(SpanDecodingError::ParentCheck))
        );
    }

    #[test]
    fn test_decode_l1_origin_check() {
        let expected = FixedBytes::<20>::from([0x42; 20]);
        let mut buf = Vec::with_capacity(expected.length());
        expected.encode(&mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_l1_origin_check(&mut r).unwrap();
        assert_eq!(prefix.l1_origin_check, expected);
    }

    #[test]
    fn test_decode_l1_origin_check_short() {
        let r = &[0x42; 19];
        let mut r = &r[..];
        let mut prefix = SpanBatchPrefix::default();
        let result = prefix.decode_l1_origin_check(&mut r);
        assert_eq!(
            result,
            Err(SpanBatchError::Decoding(SpanDecodingError::L1OriginCheck))
        );
    }
}
