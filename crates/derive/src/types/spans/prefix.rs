//! Raw Span Batch Prefix

use crate::types::spans::{SpanDecodingError, SpanBatchError};

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
        let (l1_origin_num, _) = unsigned_varint::decode::u64(r)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::L1OriginNumber))?;
        self.l1_origin_num = l1_origin_num;
        Ok(())
    }

    /// Decodes the parent check from a reader.
    pub fn decode_parent_check(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        if r.len() < 20 {
            return Err(SpanBatchError::Decoding(SpanDecodingError::ParentCheck));
        }
        self.parent_check.copy_from_slice(&r[..20]);
        *r = &r[20..];
        Ok(()) 
    }

    /// Decodes the L1 origin check from a reader.
    pub fn decode_l1_origin_check(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        if r.len() < 20 {
            return Err(SpanBatchError::Decoding(SpanDecodingError::L1OriginCheck));
        }
        self.l1_origin_check.copy_from_slice(&r[..20]);
        *r = &r[20..];
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{SpanBatchPrefix, SpanBatchError, SpanDecodingError};

    #[test]
    fn test_decode_rel_timestamp() {
        let expected = 1337;
        let mut buf = [0u8; 10];
        let _byte_slice = unsigned_varint::encode::u64(expected, &mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_rel_timestamp(&mut r).unwrap();
        assert_eq!(prefix.rel_timestamp, expected);
    }

    #[test]
    fn test_decode_l1_origin_num() {
        let expected = 1337;
        let mut buf = [0u8; 10];
        let _byte_slice = unsigned_varint::encode::u64(expected, &mut buf);
        let mut r = &buf[..];
        let mut prefix = SpanBatchPrefix::default();
        prefix.decode_l1_origin_num(&mut r).unwrap();
        assert_eq!(prefix.l1_origin_num, expected);
    }

    #[test]
    fn test_decode_parent_check() {
        let expected = [0x42; 20];
        let mut r = &expected[..];
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
        assert_eq!(result, Err(SpanBatchError::Decoding(SpanDecodingError::ParentCheck)));
    }

    #[test]
    fn test_decode_l1_origin_check() {
        let expected = [0x42; 20];
        let mut r = &expected[..];
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
        assert_eq!(result, Err(SpanBatchError::Decoding(SpanDecodingError::L1OriginCheck)));
    }
}
