//! This module contains the legacy transaction data type for a span batch.

use crate::types::{
    network::Signed, SpanBatchError, SpanDecodingError, Transaction, TxEnvelope, TxKind, TxLegacy,
};
use alloy_primitives::{Address, Signature, U256};
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

impl SpanBatchLegacyTransactionData {
    /// Converts [SpanBatchLegacyTransactionData] into a [TxEnvelope].
    pub fn to_enveloped_tx(
        &self,
        nonce: u64,
        gas: u64,
        to: Option<Address>,
        chain_id: u64,
        signature: Signature,
    ) -> Result<TxEnvelope, SpanBatchError> {
        let legacy_tx = TxLegacy {
            chain_id: Some(chain_id),
            nonce,
            gas_price: u128::from_be_bytes(
                self.gas_price.to_be_bytes::<32>()[16..]
                    .try_into()
                    .map_err(|_| {
                        SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData)
                    })?,
            ),
            gas_limit: gas,
            to: if let Some(to) = to {
                TxKind::Call(to)
            } else {
                TxKind::Create
            },
            value: self.value,
            input: self.data.clone().into(),
        };
        let signature_hash = legacy_tx.signature_hash();
        let signed_legacy_tx = Signed::new_unchecked(legacy_tx, signature, signature_hash);
        Ok(TxEnvelope::Legacy(signed_legacy_tx))
    }
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
    use super::*;
    use crate::types::SpanBatchTransactionData;
    use alloc::vec::Vec;
    // use alloy_primitives::B256;

    // #[test]
    // fn to_enveloped_tx() {
    //     let legacy_tx = SpanBatchLegacyTransactionData {
    //         value: U256::from(0xFF),
    //         gas_price: U256::from(0xEE),
    //         data: Bytes::from(alloc::vec![0x01, 0x02, 0x03]),
    //     };
    //     let nonce = 0x1234;
    //     let gas = 0x5678;
    //     let to = None;
    //     let chain_id = 0x9ABC;
    //     let signature = &[0x01; 65];
    //     let signature = Signature::decode(&mut &signature[..]).unwrap();
    //     let enveloped_tx = legacy_tx
    //         .to_enveloped_tx(nonce, gas, to, chain_id, signature)
    //         .unwrap();
    //     let expected = TxEnvelope::Legacy(crate::types::network::Signed::new_unchecked(
    //         crate::types::TxLegacy {
    //             chain_id: Some(chain_id),
    //             nonce,
    //             gas_price: 0xEE,
    //             gas_limit: gas,
    //             to: crate::types::TxKind::Create,
    //             value: U256::from(0xFF),
    //             input: Bytes::from(alloc::vec![0x01, 0x02, 0x03]).into(),
    //         },
    //         signature,
    //         B256::from([0x01; 32]),
    //     ));
    //     assert_eq!(enveloped_tx, expected);
    // }

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
