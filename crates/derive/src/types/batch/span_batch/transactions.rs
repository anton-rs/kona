//! This module contains the [SpanBatchTransactions] type and logic for encoding and decoding
//! transactions in a span batch.

use super::{
    convert_v_to_y_parity, read_tx_data, utils::is_protected_v, SpanBatchBits, SpanBatchError,
    SpanBatchSignature, SpanBatchTransactionData, SpanDecodingError,
};
use crate::types::RawTransaction;
use alloc::vec::Vec;
use alloy_consensus::{Transaction, TxEnvelope, TxType};
use alloy_eips::eip2718::Encodable2718;
use alloy_primitives::{Address, TxKind, U256};
use alloy_rlp::{Buf, Decodable, Encodable};

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
    pub tx_types: Vec<TxType>,
    /// Total legacy transaction count in the span batch.
    pub legacy_tx_count: u64,
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
        SpanBatchBits::encode(w, self.total_block_tx_count as usize, &self.contract_creation_bits)?;
        Ok(())
    }

    /// Encode the protected bits into a writer.
    pub fn encode_protected_bits(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        SpanBatchBits::encode(w, self.legacy_tx_count as usize, &self.protected_bits)?;
        Ok(())
    }

    /// Encode the y parity bits into a writer.
    pub fn encode_y_parity_bits(&self, w: &mut Vec<u8>) -> Result<(), SpanBatchError> {
        SpanBatchBits::encode(w, self.total_block_tx_count as usize, &self.y_parity_bits)?;
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
            let s_val = U256::from_be_slice(&r[32..64]);
            sigs.push(SpanBatchSignature { v: 0, r: r_val, s: s_val });
            r.advance(64);
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
            r.advance(20);
        }
        self.tx_tos = tos;
        Ok(())
    }

    /// Decode the transaction data from a reader.
    pub fn decode_tx_datas(&mut self, r: &mut &[u8]) -> Result<(), SpanBatchError> {
        let mut tx_datas = Vec::new();
        let mut tx_types = Vec::new();

        // Do not need the transaction data header because the RLP stream already includes the
        // length information.
        for _ in 0..self.total_block_tx_count {
            let (tx_data, tx_type) = read_tx_data(r)?;
            tx_datas.push(tx_data);
            tx_types.push(tx_type);
            if matches!(tx_type, TxType::Legacy) {
                self.legacy_tx_count += 1;
            }
        }

        self.tx_datas = tx_datas;
        self.tx_types = tx_types;

        Ok(())
    }

    /// Returns the number of contract creation transactions in the span batch.
    pub fn contract_creation_count(&self) -> u64 {
        self.contract_creation_bits.0.iter().map(|b| b.count_ones() as u64).sum()
    }

    /// Recover the `v` values of the transaction signatures.
    pub fn recover_v(&mut self, chain_id: u64) -> Result<(), SpanBatchError> {
        if self.tx_sigs.len() != self.tx_types.len() {
            return Err(SpanBatchError::Decoding(SpanDecodingError::TypeSignatureLenMismatch));
        }
        let mut protected_bits_idx = 0;
        for (i, tx_type) in self.tx_types.iter().enumerate() {
            let bit = self.y_parity_bits.get_bit(i).ok_or(SpanBatchError::BitfieldTooLong)?;
            let v = match tx_type {
                TxType::Legacy => {
                    // Legacy transaction
                    let protected_bit = self.protected_bits.get_bit(protected_bits_idx);
                    protected_bits_idx += 1;
                    if protected_bit.is_none() || protected_bit.is_some_and(|b| b == 0) {
                        Ok(27 + bit as u64)
                    } else {
                        // EIP-155
                        Ok(chain_id * 2 + 35 + bit as u64)
                    }
                }
                TxType::Eip2930 | TxType::Eip1559 => Ok(bit as u64),
                _ => Err(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionType)),
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
                .ok_or(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;
            let gas = self
                .tx_gases
                .get(idx as usize)
                .ok_or(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;
            let bit = self
                .contract_creation_bits
                .get_bit(idx as usize)
                .ok_or(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;
            let to = if bit == 0 {
                if self.tx_tos.len() <= to_idx {
                    return Err(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData));
                }
                to_idx += 1;
                Some(self.tx_tos[to_idx - 1])
            } else {
                None
            };
            let sig = *self
                .tx_sigs
                .get(idx as usize)
                .ok_or(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;
            let tx_envelope = tx.to_enveloped_tx(*nonce, *gas, to, chain_id, sig.try_into()?)?;
            let mut buf = Vec::new();
            tx_envelope.encode_2718(&mut buf);
            txs.push(buf);
        }
        Ok(txs)
    }

    /// Add raw transactions into the [SpanBatchTransactions].
    pub fn add_txs(
        &mut self,
        txs: Vec<RawTransaction>,
        chain_id: u64,
    ) -> Result<(), SpanBatchError> {
        let total_block_tx_count = txs.len() as u64;
        let offset = self.total_block_tx_count;

        for i in 0..total_block_tx_count {
            let tx_enveloped = TxEnvelope::decode(&mut txs[i as usize].as_ref())
                .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;
            let span_batch_tx = SpanBatchTransactionData::try_from(&tx_enveloped)?;

            let tx_type = tx_enveloped.tx_type();
            if matches!(tx_type, TxType::Legacy) {
                let protected_bit = is_protected_v(&tx_enveloped);
                self.protected_bits.set_bit(self.legacy_tx_count as usize, protected_bit);
                self.legacy_tx_count += 1;
            }

            let (signature, to, nonce, gas, tx_chain_id) = match &tx_enveloped {
                TxEnvelope::Legacy(tx) => {
                    let (tx, sig) = (tx.tx(), tx.signature());
                    (sig, tx.to(), tx.nonce(), tx.gas_limit(), tx.chain_id())
                }
                TxEnvelope::Eip2930(tx) => {
                    let (tx, sig) = (tx.tx(), tx.signature());
                    (sig, tx.to(), tx.nonce(), tx.gas_limit(), tx.chain_id())
                }
                TxEnvelope::Eip1559(tx) => {
                    let (tx, sig) = (tx.tx(), tx.signature());
                    (sig, tx.to(), tx.nonce(), tx.gas_limit(), tx.chain_id())
                }
                _ => {
                    return Err(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))
                }
            };

            if is_protected_v(&tx_enveloped) &&
                tx_chain_id
                    .ok_or(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))? !=
                    chain_id
            {
                return Err(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData));
            }

            let signature_v = signature.v().to_u64();
            let y_parity_bit = convert_v_to_y_parity(signature_v, tx_type)?;
            let contract_creation_bit = match to {
                TxKind::Call(address) => {
                    self.tx_tos.push(address);
                    0
                }
                TxKind::Create => 1,
            };
            let mut tx_data_buf = Vec::new();
            span_batch_tx.encode(&mut tx_data_buf);

            self.tx_sigs.push((*signature).into());
            self.contract_creation_bits.set_bit((i + offset) as usize, contract_creation_bit == 1);
            self.y_parity_bits.set_bit((i + offset) as usize, y_parity_bit);
            self.tx_nonces.push(nonce);
            self.tx_datas.push(tx_data_buf);
            self.tx_gases.push(gas.try_into().map_err(|_| {
                SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData)
            })?);
            self.tx_types.push(tx_type);
        }
        self.total_block_tx_count += total_block_tx_count;
        Ok(())
    }
}
