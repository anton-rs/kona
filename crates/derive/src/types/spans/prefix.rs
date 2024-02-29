//! Raw Span Batch Prefix

use crate::types::spans::{SpanBatchError, SpanDecodingError};
use alloc::vec::Vec;
use alloy_rlp::{Decodable, Encodable};

/// Span Batch Prefix
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanBatchPrefix {
    /// Relative timestamp of the first block
    pub rel_timestamp: u64,
    /// L1 origin number
    pub l1_origin_num: u64,
    /// First 20 bytes of the first block's parent hash
    pub parent_check: [u8; 20],
    /// First 20 bytes of the last block's L1 origin hash
    pub l1_origin_check: [u8; 20],
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
        let mut buf = [0u8; 10];
        unsigned_varint::encode::u64(self.rel_timestamp, &mut buf);
        out.put_slice(&buf[..]);
        let mut buf = [0u8; 10];
        unsigned_varint::encode::u64(self.l1_origin_num, &mut buf);
        out.put_slice(&buf[..]);
        self.parent_check.encode(out);
        self.l1_origin_check.encode(out);
        // alloy_rlp::Bytes::encode(&self.parent_check[..], out);
        // out.put_slice(&self.parent_check);
        // out.put_slice(&self.l1_origin_check);
        // FixedBytes::<20>::encode(&self.parent_check, out);
        // FixedBytes::<20>::encode(&self.l1_origin_check, out);

        // out.put_u64(self.rel_timestamp);
        // out.put_u64(self.l1_origin_num);
        // self.parent_check.encode(out);
        // self.l1_origin_check.encode(out);
    }
}

impl SpanBatchPrefix {
    /// Decodes the relative timestamp from a reader.
    pub fn decode_rel_timestamp(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let (rel_timestamp, _) = unsigned_varint::decode::u64(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::RelativeTimestamp))?;
        self.rel_timestamp = rel_timestamp;
        Ok(())
    }

    /// Decodes the L1 origin number from a reader.
    pub fn decode_l1_origin_num(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let (l1_origin_num, _) = unsigned_varint::decode::u64(r).map_err(|_| SpanBatchError::Decoding(SpanDecodingError::L1OriginNumber))?;
        self.l1_origin_num = l1_origin_num;
        Ok(())
    }

    /// Decodes the parent check from a reader.
    pub fn decode_parent_check(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let parent_check = alloy_rlp::Bytes::decode(r).map_err(|_| SpanBatchError::Decoding(SpanDecodingError::ParentCheck))?;
        let mut parent_check_fixed = [0u8; 20];
        parent_check_fixed.copy_from_slice(&parent_check);
        self.parent_check = parent_check_fixed;
        Ok(())
        // let parent_check = r[..20].try_into().map_err(|_| SpanBatchError::Decoding(SpanDecodingError::ParentCheck))?;
        // self.parent_check = parent_check;
        // Ok(())
    }

    /// Decodes the L1 origin check from a reader.
    pub fn decode_l1_origin_check(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let l1_origin_check = alloy_rlp::Bytes::decode(r).map_err(|_| SpanBatchError::Decoding(SpanDecodingError::L1OriginCheck))?;
        let mut origin_check = [0u8; 20];
        origin_check.copy_from_slice(&l1_origin_check);
        self.l1_origin_check = origin_check;
        Ok(())
    }

    /// Returns the prefix encoded into a byte vec.
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode(&mut buf);
        buf
    }
}

#[cfg(test)]
mod test {
    use super::SpanBatchPrefix;
    use alloc::vec::Vec;
    use alloy_rlp::{Decodable, Encodable};

    #[test]
    fn test_span_batch_prefix_roundtrip_encoding() {
        let expected = SpanBatchPrefix {
            rel_timestamp: 1337,
            l1_origin_num: 42,
            parent_check: [0x42; 20],
            l1_origin_check: [0x42; 20],
        };
        let encoded = expected.encode_to_vec();
        let result = SpanBatchPrefix::decode(&mut &encoded[..]);
        assert_eq!(result, Ok(expected));
    }

    #[test]
    fn test_decode_rel_timestamp() {
        let expected = 1337;
        let mut buf = [0u8; 10];
        unsigned_varint::encode::u64(expected, &mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_rel_timestamp(&mut r).unwrap();
        assert_eq!(prefix.rel_timestamp, expected);
    }

    #[test]
    fn test_decode_l1_origin_num() {
        let expected = 1337;
        let mut buf = [0u8; 10];
        unsigned_varint::encode::u64(expected, &mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_l1_origin_num(&mut r).unwrap();
        assert_eq!(prefix.l1_origin_num, expected);
    }

    #[test]
    fn test_decode_parent_check() {
        let expected = [0x42; 20];
        let mut buf = Vec::with_capacity(expected.length());
        expected.encode(&mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_parent_check(&mut r).unwrap();
        assert_eq!(prefix.parent_check, expected);
    }

    #[test]
    fn test_decode_l1_origin_check() {
        let expected = [0x42; 20];
        let mut buf = Vec::new();
        expected.encode(&mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_l1_origin_check(&mut r).unwrap();
        assert_eq!(prefix.l1_origin_check, expected);
    }
}
