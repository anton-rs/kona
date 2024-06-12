//! Contains the [BatchType] and its encodings.

use alloy_rlp::{Decodable, Encodable};

/// The single batch type identifier.
pub(crate) const SINGLE_BATCH_TYPE: u8 = 0x00;

/// The span batch type identifier.
pub(crate) const SPAN_BATCH_TYPE: u8 = 0x01;

/// The Batch Type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum BatchType {
    /// Single Batch.
    Single = SINGLE_BATCH_TYPE,
    /// Span Batch.
    Span = SPAN_BATCH_TYPE,
}

impl From<u8> for BatchType {
    fn from(val: u8) -> Self {
        match val {
            SINGLE_BATCH_TYPE => BatchType::Single,
            SPAN_BATCH_TYPE => BatchType::Span,
            _ => panic!("Invalid batch type: {val}"),
        }
    }
}

impl From<&[u8]> for BatchType {
    fn from(buf: &[u8]) -> Self {
        BatchType::from(buf[0])
    }
}

impl Encodable for BatchType {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        let val = match self {
            BatchType::Single => SINGLE_BATCH_TYPE,
            BatchType::Span => SPAN_BATCH_TYPE,
        };
        val.encode(out);
    }
}

impl Decodable for BatchType {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let val = u8::decode(buf)?;
        Ok(BatchType::from(val))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn test_batch_type_rlp_roundtrip() {
        let batch_type = BatchType::Single;
        let mut buf = Vec::new();
        batch_type.encode(&mut buf);
        let decoded = BatchType::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(batch_type, decoded);
    }
}
