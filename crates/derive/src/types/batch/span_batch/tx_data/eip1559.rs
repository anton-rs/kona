//! This module contains the eip1559 transaction data type for a span batch.

use crate::types::eip2930::AccessList;
use alloy_primitives::U256;
use alloy_rlp::{Bytes, Decodable, Encodable, Header};

/// The transaction data for an EIP-1559 transaction within a span batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatchEip1559TransactionData {
    /// The ETH value of the transaction.
    pub value: U256,
    /// Maximum fee per gas.
    pub max_fee_per_gas: U256,
    /// Maximum priority fee per gas.
    pub max_priority_fee_per_gas: U256,
    /// Transaction calldata.
    pub data: Bytes,
    /// Access list, used to pre-warm storage slots through static declaration.
    pub access_list: AccessList,
}

impl Encodable for SpanBatchEip1559TransactionData {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        let payload_length = self.value.length()
            + self.max_fee_per_gas.length()
            + self.max_priority_fee_per_gas.length()
            + self.data.length()
            + self.access_list.length();
        let header = Header {
            list: true,
            payload_length,
        };

        header.encode(out);
        self.value.encode(out);
        self.max_fee_per_gas.encode(out);
        self.max_priority_fee_per_gas.encode(out);
        self.data.encode(out);
        self.access_list.encode(out);
    }
}

impl Decodable for SpanBatchEip1559TransactionData {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::Custom(
                "Expected list data for EIP-1559 transaction",
            ));
        }
        let buf_len_start = buf.len();

        let value = U256::decode(buf)?;
        let max_fee_per_gas = U256::decode(buf)?;
        let max_priority_fee_per_gas = U256::decode(buf)?;
        let data = Bytes::decode(buf)?;
        let access_list = AccessList::decode(buf)?;

        if buf.len() != buf_len_start - header.payload_length {
            return Err(alloy_rlp::Error::Custom("Invalid EIP-1559 transaction RLP"));
        }

        Ok(Self {
            value,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            data,
            access_list,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::SpanBatchTransactionData;
    use alloc::vec::Vec;

    #[test]
    fn encode_eip1559_tx_data_roundtrip() {
        let variable_fee_tx = SpanBatchEip1559TransactionData {
            value: U256::from(0xFF),
            max_fee_per_gas: U256::from(0xEE),
            max_priority_fee_per_gas: U256::from(0xDD),
            data: Bytes::from(alloc::vec![0x01, 0x02, 0x03]),
            access_list: AccessList::default(),
        };
        let mut encoded_buf = Vec::new();
        SpanBatchTransactionData::Eip1559(variable_fee_tx.clone()).encode(&mut encoded_buf);

        let decoded = SpanBatchTransactionData::decode(&mut encoded_buf.as_slice()).unwrap();
        let SpanBatchTransactionData::Eip1559(variable_fee_decoded) = decoded else {
            panic!(
                "Expected SpanBatchEip1559TransactionData, got {:?}",
                decoded
            );
        };

        assert_eq!(variable_fee_tx, variable_fee_decoded);
    }
}
