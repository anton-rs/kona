//! This module contains the legacy transaction data type for a span batch.

use alloy_primitives::U256;
use alloy_rlp::{Bytes, Decodable, Encodable, Header};

/// The transaction data for a legacy transaction within a span batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatchLegacyTransactionData {
    /// The ETH value of the transaction.
    pub value: U256,
    /// The gas price of the transaction.
    pub gas_price: U256,
    /// Transaction calldata.
    pub data: Bytes,
}

impl Encodable for SpanBatchLegacyTransactionData {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        let payload_length = self.value.length() + self.gas_price.length() + self.data.length();
        let header = Header {
            list: true,
            payload_length,
        };

        header.encode(out);
        self.value.encode(out);
        self.gas_price.encode(out);
        self.data.encode(out);
    }
}

impl Decodable for SpanBatchLegacyTransactionData {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::Custom(
                "Expected list data for Legacy transaction",
            ));
        }
        let buf_len_start = buf.len();

        let value = U256::decode(buf)?;
        let gas_price = U256::decode(buf)?;
        let data = Bytes::decode(buf)?;

        if buf.len() != buf_len_start - header.payload_length {
            return Err(alloy_rlp::Error::Custom("Invalid Legacy transaction RLP"));
        }

        Ok(Self {
            value,
            gas_price,
            data,
        })
    }
}

#[cfg(test)]
mod test {
    use super::SpanBatchLegacyTransactionData;
    use crate::types::SpanBatchTransactionData;
    use alloc::vec::Vec;
    use alloy_primitives::U256;
    use alloy_rlp::{Bytes, Decodable, Encodable};

    #[test]
    fn encode_legacy_tx_data_roundtrip() {
        let legacy_tx = SpanBatchLegacyTransactionData {
            value: U256::from(0xFF),
            gas_price: U256::from(0xEE),
            data: Bytes::from(alloc::vec![0x01, 0x02, 0x03]),
        };

        let mut encoded_buf = Vec::new();
        SpanBatchTransactionData::Legacy(legacy_tx.clone()).encode(&mut encoded_buf);

        let decoded = SpanBatchTransactionData::decode(&mut encoded_buf.as_slice()).unwrap();
        let SpanBatchTransactionData::Legacy(legacy_decoded) = decoded else {
            panic!("Expected SpanBatchLegacyTransactionData, got {:?}", decoded);
        };

        assert_eq!(legacy_tx, legacy_decoded);
    }
}
