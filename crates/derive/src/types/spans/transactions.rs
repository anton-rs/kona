//! This module contains the [SpanBatchTransactions] type and logic for encoding and decoding transactions in a span batch.

use alloc::vec::Vec;
use alloy_primitives::{Address, Signature, U256};
use alloy_rlp::{Buf, Decodable, Encodable, Header, RlpDecodable, RlpEncodable};

use crate::types::{
    eip2930::AccessList, network::Signed, Transaction, TxEip1559, TxEip2930, TxEnvelope, TxKind,
    TxLegacy,
};

use super::{SpanBatchBits, SpanBatchError, SpanDecodingError};

/// This struct contains the decoded information for transactions in a span batch.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SpanBatchTransactions {
    /// The total number of transactions in a span batch. Must be manually set.
    pub total_block_tx_count: u64,
    /// The contract creation bits, standard span-batch bitlist.
    pub contract_creation_bits: SpanBatchBits,
    /// The y parity bits, standard span-batch bitlist.
    pub y_parity_bits: SpanBatchBits,
    /// The transaction signatures.
    pub tx_sigs: Vec<SpanBatchSignature>,
    /// The transaction nonces
    pub tx_nonces: Vec<u64>,
    /// The transaction gas limits.
    pub tx_gases: Vec<u64>,
    /// The `to` addresses of the transactions.
    pub tx_tos: Vec<Address>,
    /// The transaction data.
    pub tx_datas: Vec<Vec<u8>>,
    /// The protected bits, standard span-batch bitlist.
    pub protected_bits: SpanBatchBits,
    /// The types of the transactions.
    pub tx_types: Vec<u64>,
    /// Total legacy transaction count in the span batch.
    pub legacy_tx_count: u64,
}

/// The ECDSA signature of a transaction within a span batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanBatchSignature {
    v: u64,
    r: U256,
    s: U256,
}

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

/// The transaction data for a legacy transaction within a span batch.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct SpanBatchLegacyTransactionData {
    /// The ETH value of the transaction.
    pub value: U256,
    /// The gas price of the transaction.
    pub gas_price: U256,
    /// Transaction calldata.
    pub data: Vec<u8>,
}

/// The transaction data for an EIP-2930 transaction within a span batch.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct SpanBatchEip2930TransactionData {
    /// The ETH value of the transaction.
    pub value: U256,
    /// The gas price of the transaction.
    pub gas_price: U256,
    /// Transaction calldata.
    pub data: Vec<u8>,
    /// Access list, used to pre-warm storage slots through static declaration.
    pub access_list: AccessList,
}

/// The transaction data for an EIP-1559 transaction within a span batch.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct SpanBatchEip1559TransactionData {
    /// The ETH value of the transaction.
    pub value: U256,
    /// Maximum fee per gas.
    pub max_fee_per_gas: U256,
    /// Maximum priority fee per gas.
    pub max_priority_fee_per_gas: U256,
    /// Transaction calldata.
    pub data: Vec<u8>,
    /// Access list, used to pre-warm storage slots through static declaration.
    pub access_list: AccessList,
}

impl SpanBatchTransactions {
    /// Encodes the [SpanBatchTransactions] into a writer.
    pub fn encode(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        self.encode_contract_creation_bits(w)?;
        self.encode_y_parity_bits(w)?;
        self.encode_tx_sigs_rs(w)?;
        self.encode_tx_tos(w)?;
        self.encode_tx_datas(w)?;
        self.encode_tx_nonces(w)?;
        self.encode_tx_gases(w)?;
        self.encode_protected_bits(w)?;
        Ok(())
    }

    /// Decodes the [SpanBatchTransactions] from a reader.
    pub fn decode(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        self.decode_contract_creation_bits(r)?;
        self.decode_y_parity_bits(r)?;
        self.decode_tx_sigs_rs(r)?;
        self.decode_tx_tos(r)?;
        self.decode_tx_datas(r)?;
        self.decode_tx_nonces(r)?;
        self.decode_tx_gases(r)?;
        self.decode_protected_bits(r)?;
        Ok(())
    }

    /// Encode the contract creation bits into a writer.
    pub fn encode_contract_creation_bits(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        SpanBatchBits::encode(
            w,
            self.total_block_tx_count as usize,
            self.contract_creation_bits.as_ref(),
        )?;
        Ok(())
    }

    /// Encode the protected bits into a writer.
    pub fn encode_protected_bits(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        SpanBatchBits::encode(
            w,
            self.legacy_tx_count as usize,
            self.protected_bits.as_ref(),
        )?;
        Ok(())
    }

    /// Encode the y parity bits into a writer.
    pub fn encode_y_parity_bits(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        SpanBatchBits::encode(
            w,
            self.total_block_tx_count as usize,
            self.y_parity_bits.as_ref(),
        )?;
        Ok(())
    }

    /// Encode the transaction signatures into a writer (excluding `v` field).
    pub fn encode_tx_sigs_rs(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        for sig in &self.tx_sigs {
            w.extend_from_slice(&sig.r.to_be_bytes::<32>());
            w.extend_from_slice(&sig.s.to_be_bytes::<32>());
        }
        Ok(())
    }

    /// Encode the transaction nonces into a writer.
    pub fn encode_tx_nonces(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        let mut buf = [0u8; 10];
        for nonce in &self.tx_nonces {
            let slice = unsigned_varint::encode::u64(*nonce, &mut buf);
            w.extend_from_slice(slice);
        }
        Ok(())
    }

    /// Encode the transaction gas limits into a writer.
    pub fn encode_tx_gases(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        let mut buf = [0u8; 10];
        for gas in &self.tx_gases {
            let slice = unsigned_varint::encode::u64(*gas, &mut buf);
            w.extend_from_slice(slice);
        }
        Ok(())
    }

    /// Encode the `to` addresses of the transactions into a writer.
    pub fn encode_tx_tos(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        for to in &self.tx_tos {
            w.extend_from_slice(to.as_ref());
        }
        Ok(())
    }

    /// Encode the transaction data into a writer.
    pub fn encode_tx_datas(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        for data in &self.tx_datas {
            w.extend_from_slice(data);
        }
        Ok(())
    }

    /// Decode the contract creation bits from a reader.
    pub fn decode_contract_creation_bits(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        self.contract_creation_bits = SpanBatchBits::decode(r, self.total_block_tx_count as usize)?;
        Ok(())
    }

    /// Decode the protected bits from a reader.
    pub fn decode_protected_bits(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        self.protected_bits = SpanBatchBits::decode(r, self.legacy_tx_count as usize)?;
        Ok(())
    }

    /// Decode the y parity bits from a reader.
    pub fn decode_y_parity_bits(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        self.y_parity_bits = SpanBatchBits::decode(r, self.total_block_tx_count as usize)?;
        Ok(())
    }

    /// Decode the transaction signatures from a reader (excluding `v` field).
    pub fn decode_tx_sigs_rs(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let mut sigs = Vec::with_capacity(self.total_block_tx_count as usize);
        for _ in 0..self.total_block_tx_count {
            let r_val = U256::from_be_slice(&r[..32]);
            let s_val = U256::from_be_slice(&r[32..]);
            sigs.push(SpanBatchSignature {
                v: 0,
                r: r_val,
                s: s_val,
            });
            *r = &r[64..];
        }
        self.tx_sigs = sigs;
        Ok(())
    }

    /// Decode the transaction nonces from a reader.
    pub fn decode_tx_nonces(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let mut nonces = Vec::with_capacity(self.total_block_tx_count as usize);
        for _ in 0..self.total_block_tx_count {
            let (nonce, remaining) = unsigned_varint::decode::u64(r)
                .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::TxNonces))?;
            nonces.push(nonce);
            *r = remaining;
        }
        self.tx_nonces = nonces;
        Ok(())
    }

    /// Decode the transaction gas limits from a reader.
    pub fn decode_tx_gases(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let mut gases = Vec::with_capacity(self.total_block_tx_count as usize);
        for _ in 0..self.total_block_tx_count {
            let (gas, remaining) = unsigned_varint::decode::u64(r)
                .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::TxNonces))?;
            gases.push(gas);
            *r = remaining;
        }
        self.tx_gases = gases;
        Ok(())
    }

    /// Decode the `to` addresses of the transactions from a reader.
    pub fn decode_tx_tos(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let mut tos = Vec::with_capacity(self.total_block_tx_count as usize);
        let contract_creation_count = self.contract_creation_count();
        for _ in 0..(self.total_block_tx_count - contract_creation_count) {
            let to = Address::from_slice(&r[..20]);
            tos.push(to);
            *r = &r[20..];
        }
        self.tx_tos = tos;
        Ok(())
    }

    /// Decode the transaction data from a reader.
    pub fn decode_tx_datas(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let mut tx_datas = Vec::new();
        let mut tx_types = Vec::new();

        // Do not need the transaction data header because the RLP stream already includes the length information.
        for _ in 0..self.total_block_tx_count {
            let (tx_data, tx_type) = read_tx_data(r)?;
            tx_datas.push(tx_data);
            tx_types.push(tx_type as u64);
            if tx_type == 0 {
                self.legacy_tx_count += 1;
            }
        }

        self.tx_datas = tx_datas;
        self.tx_types = tx_types;

        Ok(())
    }

    /// Returns the number of contract creation transactions in the span batch.
    pub fn contract_creation_count(&self) -> u64 {
        self.contract_creation_bits
            .0
            .iter()
            .map(|b| b.count_ones() as u64)
            .sum()
    }

    /// Recover the `v` values of the transaction signatures.
    pub fn recover_v(&mut self, chain_id: u64) -> Result<(), SpanBatchError> {
        if self.tx_sigs.len() != self.tx_types.len() {
            return Err(SpanBatchError::Decoding(
                SpanDecodingError::TypeSignatureLenMismatch,
            ));
        }
        let mut protected_bits_idx = 0;
        for (i, tx_type) in self.tx_types.iter().enumerate() {
            let bit = self
                .y_parity_bits
                .get_bit(i)
                .ok_or(SpanBatchError::BitfieldTooLong)?;
            let v = match tx_type {
                0 => {
                    // Legacy transaction
                    let protected_bit = self
                        .protected_bits
                        .get_bit(protected_bits_idx)
                        .ok_or(SpanBatchError::BitfieldTooLong)?;
                    protected_bits_idx += 1;
                    if protected_bit == 0 {
                        Ok(27 + bit as u64)
                    } else {
                        // EIP-155
                        Ok(chain_id * 2 + 35 + bit as u64)
                    }
                }
                1 | 2 => {
                    // EIP-2930 + EIP-1559
                    Ok(bit as u64)
                }
                _ => Err(SpanBatchError::Decoding(
                    SpanDecodingError::InvalidTransactionType,
                )),
            }?;
            self.tx_sigs.get_mut(i).expect("Transaction must exist").v = v;
        }
        Ok(())
    }

    /// Retrieve all of the raw transactions from the [SpanBatchTransactions].
    pub fn full_txs(&self, chain_id: u64) -> Result<Vec<Vec<u8>>, SpanBatchError> {
        let mut txs = Vec::new();
        let mut to_idx = 0;
        for idx in 0..self.total_block_tx_count {
            let mut datas = self.tx_datas[idx as usize].as_slice();
            let tx = SpanBatchTransactionData::decode(&mut datas)
                .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;
            let nonce = self
                .tx_nonces
                .get(idx as usize)
                .ok_or(SpanBatchError::Decoding(
                    SpanDecodingError::InvalidTransactionData,
                ))?;
            let gas = self
                .tx_gases
                .get(idx as usize)
                .ok_or(SpanBatchError::Decoding(
                    SpanDecodingError::InvalidTransactionData,
                ))?;
            let bit = self.contract_creation_bits.get_bit(idx as usize).ok_or(
                SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData),
            )?;
            let to = if bit == 0 {
                if self.tx_tos.len() <= to_idx {
                    return Err(SpanBatchError::Decoding(
                        SpanDecodingError::InvalidTransactionData,
                    ));
                }
                to_idx += 1;
                Some(self.tx_tos[to_idx - 1])
            } else {
                None
            };
            let sig = *self
                .tx_sigs
                .get(idx as usize)
                .ok_or(SpanBatchError::Decoding(
                    SpanDecodingError::InvalidTransactionData,
                ))?;
            let tx_envelope = tx.to_enveloped_tx(*nonce, *gas, to, chain_id, sig.try_into()?)?;
            let mut buf = Vec::new();
            tx_envelope.encode(&mut buf);
            txs.push(buf);
        }
        Ok(txs)
    }

    /// Add raw transactions into the [SpanBatchTransactions].
    pub fn add_txs(&mut self, _txs: Vec<Vec<u8>>, _chain_id: u64) -> Result<(), SpanBatchError> {
        // let total_block_tx_count = txs.len() as u64;
        // let offset = self.total_block_tx_count;
        //
        // for i in 0..total_block_tx_count {
        //     // todo - RLP decode transaction and extract fields.
        // }
        todo!()
    }
}

impl Encodable for SpanBatchTransactionData {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Self::Legacy(data) => {
                data.encode(out);
            }
            Self::Eip2930(data) => {
                data.encode(out);
            }
            Self::Eip1559(data) => {
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

impl TryFrom<SpanBatchSignature> for Signature {
    type Error = SpanBatchError;

    fn try_from(value: SpanBatchSignature) -> Result<Self, Self::Error> {
        Self::from_rs_and_parity(value.r, value.s, convert_v_to_y_parity(value.v, 0)?)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionSignature))
    }
}

/// Reads transaction data from a reader.
pub(crate) fn read_tx_data(r: &mut &[u8]) -> Result<(Vec<u8>, u8), SpanBatchError> {
    let mut tx_data = Vec::new();
    let first_byte = *r.first().ok_or(SpanBatchError::Decoding(
        SpanDecodingError::InvalidTransactionData,
    ))?;
    let mut tx_type = 0;
    if first_byte <= 0x7F {
        // EIP-2718: Non-legacy tx, so write tx type
        tx_type = first_byte;
        tx_data.push(tx_type);
        r.advance(1);
    }

    // Copy the reader, as we need to read the header to determine if the payload is a list.
    // TODO(clabby): This is horribly inefficient. It'd be nice if we could peek at this rather than forcibly having to
    // advance the buffer passed, should read more into the alloy rlp docs to see if this is possible.
    let r_copy = Vec::from(*r);
    let rlp_header = Header::decode(&mut r_copy.as_slice())
        .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;

    let tx_payload = if rlp_header.list {
        // Grab the raw RLP for the transaction data from `r`. It was unaffected since we copied it.
        let payload_length_with_header = rlp_header.payload_length + rlp_header.length();
        let payload = r[0..payload_length_with_header].to_vec();
        r.advance(payload_length_with_header);
        Ok(payload)
    } else {
        Err(SpanBatchError::Decoding(
            SpanDecodingError::InvalidTransactionData,
        ))
    }?;
    tx_data.extend_from_slice(&tx_payload);

    Ok((tx_data, tx_type))
}

/// Converts a `v` value to a y parity bit, from the transaaction type.
pub(crate) fn convert_v_to_y_parity(v: u64, tx_type: u64) -> Result<bool, SpanBatchError> {
    match tx_type {
        0 => {
            if v != 27 && v != 28 {
                // EIP-155: v = 2 * chain_id + 35 + yParity
                Ok((v - 35) & 1 == 1)
            } else {
                // Unprotected legacy txs must have v = 27 or 28
                Ok(v - 27 == 1)
            }
        }
        1 | 2 => Ok(v == 1),
        _ => Err(SpanBatchError::Decoding(
            SpanDecodingError::InvalidTransactionType,
        )),
    }
}
