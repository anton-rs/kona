//! This module contains the top level span batch transaction data type.

use super::{
    SpanBatchEip1559TransactionData, SpanBatchEip2930TransactionData,
    SpanBatchLegacyTransactionData,
};
use crate::types::{
    network::Signed, SpanBatchError, SpanDecodingError, Transaction, TxEip1559, TxEip2930,
    TxEnvelope, TxKind, TxLegacy,
};
use alloy_primitives::{Address, Signature, U256};
use alloy_rlp::{Bytes, Decodable, Encodable};

/// The typed transaction data for a transaction within a span batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanBatchTransactionData {
    /// Legacy transaction data.
    Legacy(SpanBatchLegacyTransactionData),
    /// EIP-2930 transaction data.
    Eip2930(SpanBatchEip2930TransactionData),
    /// EIP-1559 transaction data.
    Eip1559(SpanBatchEip1559TransactionData),
}

impl Encodable for SpanBatchTransactionData {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Self::Legacy(data) => {
                data.encode(out);
            }
            Self::Eip2930(data) => {
                out.put_u8(1);
                data.encode(out);
            }
            Self::Eip1559(data) => {
                out.put_u8(2);
                data.encode(out);
            }
        }
    }
}

impl Decodable for SpanBatchTransactionData {
    fn decode(r: &mut &[u8]) -> Result<Self, alloy_rlp::Error> {
        if !r.is_empty() && r[0] > 0x7F {
            // Legacy transaction
            return Ok(SpanBatchTransactionData::Legacy(
                SpanBatchLegacyTransactionData::decode(r)?,
            ));
        }
        // Non-legacy transaction (EIP-2718 envelope encoding)
        Self::decode_typed(r)
    }
}

impl TryFrom<&TxEnvelope> for SpanBatchTransactionData {
    type Error = SpanBatchError;

    fn try_from(tx_envelope: &TxEnvelope) -> Result<Self, Self::Error> {
        match tx_envelope {
            TxEnvelope::Legacy(s) => Ok(SpanBatchTransactionData::Legacy(
                SpanBatchLegacyTransactionData {
                    value: s.value,
                    gas_price: U256::from(s.gas_price),
                    data: Bytes::from(s.input().to_vec()),
                },
            )),
            TxEnvelope::Eip2930(s) => Ok(SpanBatchTransactionData::Eip2930(
                SpanBatchEip2930TransactionData {
                    value: s.value,
                    gas_price: U256::from(s.gas_price),
                    data: Bytes::from(s.input().to_vec()),
                    access_list: s.access_list.clone(),
                },
            )),
            TxEnvelope::Eip1559(s) => Ok(SpanBatchTransactionData::Eip1559(
                SpanBatchEip1559TransactionData {
                    value: s.value,
                    max_fee_per_gas: U256::from(s.max_fee_per_gas),
                    max_priority_fee_per_gas: U256::from(s.max_priority_fee_per_gas),
                    data: Bytes::from(s.input().to_vec()),
                    access_list: s.access_list.clone(),
                },
            )),
            _ => Err(SpanBatchError::Decoding(
                SpanDecodingError::InvalidTransactionType,
            )),
        }
    }
}

impl SpanBatchTransactionData {
    /// Returns the transaction type of the [SpanBatchTransactionData].
    pub fn tx_type(&self) -> u8 {
        match self {
            Self::Legacy(_) => 0,
            Self::Eip2930(_) => 1,
            Self::Eip1559(_) => 2,
        }
    }

    /// Decodes a typed transaction into a [SpanBatchTransactionData] from a byte slice.
    pub fn decode_typed(b: &[u8]) -> Result<Self, alloy_rlp::Error> {
        if b.len() <= 1 {
            return Err(alloy_rlp::Error::Custom("Invalid transaction data"));
        }

        match b[0] {
            1 => Ok(SpanBatchTransactionData::Eip2930(
                SpanBatchEip2930TransactionData::decode(&mut &b[1..])?,
            )),
            2 => Ok(SpanBatchTransactionData::Eip1559(
                SpanBatchEip1559TransactionData::decode(&mut &b[1..])?,
            )),
            _ => Err(alloy_rlp::Error::Custom("Invalid transaction type")),
        }
    }

    /// Converts the [SpanBatchTransactionData] into a [TxEnvelope].
    pub fn to_enveloped_tx(
        &self,
        nonce: u64,
        gas: u64,
        to: Option<Address>,
        chain_id: u64,
        signature: Signature,
    ) -> Result<TxEnvelope, SpanBatchError> {
        match self {
            Self::Legacy(data) => {
                let legacy_tx = TxLegacy {
                    chain_id: Some(chain_id),
                    nonce,
                    gas_price: u128::from_be_bytes(
                        data.gas_price.to_be_bytes::<32>()[16..]
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
                    value: data.value,
                    input: data.data.clone().into(),
                };
                let signature_hash = legacy_tx.signature_hash();
                let signed_legacy_tx = Signed::new_unchecked(legacy_tx, signature, signature_hash);
                Ok(TxEnvelope::Legacy(signed_legacy_tx))
            }
            Self::Eip2930(data) => {
                let access_list_tx = TxEip2930 {
                    chain_id,
                    nonce,
                    gas_price: u128::from_be_bytes(
                        data.gas_price.to_be_bytes::<32>()[16..]
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
                    value: data.value,
                    input: data.data.clone().into(),
                    access_list: data.access_list.clone(),
                };
                let signature_hash = access_list_tx.signature_hash();
                let signed_access_list_tx =
                    Signed::new_unchecked(access_list_tx, signature, signature_hash);
                Ok(TxEnvelope::Eip2930(signed_access_list_tx))
            }
            Self::Eip1559(data) => {
                let eip1559_tx = TxEip1559 {
                    chain_id,
                    nonce,
                    max_fee_per_gas: u128::from_be_bytes(
                        data.max_fee_per_gas.to_be_bytes::<32>()[16..]
                            .try_into()
                            .map_err(|_| {
                                SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData)
                            })?,
                    ),
                    max_priority_fee_per_gas: u128::from_be_bytes(
                        data.max_priority_fee_per_gas.to_be_bytes::<32>()[16..]
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
                    value: data.value,
                    input: data.data.clone().into(),
                    access_list: data.access_list.clone(),
                };
                let signature_hash = eip1559_tx.signature_hash();
                let signed_eip1559_tx =
                    Signed::new_unchecked(eip1559_tx, signature, signature_hash);
                Ok(TxEnvelope::Eip1559(signed_eip1559_tx))
            }
        }
    }
}
