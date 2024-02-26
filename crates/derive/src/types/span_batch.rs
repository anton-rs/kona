use alloc::vec::Vec;
use alloy_primitives::{Address, Bytes, StorageKey, U256};
use alloy_rlp::{BufMut, BytesMut, Decodable, Encodable};

use super::{BlockInput, RawTransaction, RollupConfig};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AccessList(pub Vec<AccessListItem>);

impl Encodable for AccessList {
    fn length(&self) -> usize {
        let mut len = 0;
        for item in &self.0 {
            len += item.address.length();
            len += item.storage_keys.len() * 32;
        }
        len
    }

    fn encode(&self, out: &mut dyn BufMut) {
        for item in &self.0 {
            item.address.encode(out);
            for key in &item.storage_keys {
                key.encode(out);
            }
        }
    }
}

impl Decodable for AccessList {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let mut items = Vec::new();
        while !buf.is_empty() {
            let address = Address::decode(buf)?;
            let mut storage_keys = Vec::new();
            for _ in 0..buf.len() / 32 {
                let key = StorageKey::decode(buf)?;
                storage_keys.push(key);
            }
            items.push(AccessListItem {
                address,
                storage_keys,
            });
        }

        Ok(Self(items))
    }
}

#[derive(Debug, Clone)]
pub struct AccessListItem {
    /// Account addresses that would be loaded at the start of execution
    pub address: Address,
    /// Keys of storage that would be loaded at the start of execution
    pub storage_keys: Vec<StorageKey>,
}

/// Represents a span batch: a range of encoded L2 blocks
#[derive(Debug, Clone)]
pub struct SpanBatch {
    /// Uvarint encoded relative timestamp since L2 genesis
    pub rel_timestamp: u64,
    /// Uvarint encoded L1 origin number of the last L2 block in the batch
    pub l1_origin_num: u64,
    /// First 20 bytes of the parent hash of the first L2 block in the batch.
    pub parent_check: [u8; 20],
    /// Last 20 bytes of the L1 origin hash of the last L2 block in the batch.
    pub l1_origin_check: [u8; 20],
    /// Uvarint encoded number of L2 blocks in the batch.
    pub block_count: u64,
    /// Bitlist of [SpanBatch.block_count] bits: 1 bit per block.
    pub origin_bits: Vec<bool>,
    /// Uvarint encoded number of L2 transactions in this batch
    pub block_tx_counts: Vec<u64>,
    /// The L2 transactions in this batch
    pub transactions: Vec<RawTransaction>,
    /// The L1 block number this batch was derived from.
    pub l1_inclusion_block: u64,
}

impl SpanBatch {
    /// Decodes a sequence of bytes into a [SpanBatch]
    pub fn decode(data: &[u8], l1_inclusion_block: u64, chain_id: u64) -> alloy_rlp::Result<Self> {
        let (rel_timestamp, data) = unsigned_varint::decode::u64(data)
            .map_err(|_| alloy_rlp::Error::Custom("Failed to decode timestamp"))?;
        let (l1_origin_num, data) = unsigned_varint::decode::u64(data)
            .map_err(|_| alloy_rlp::Error::Custom("Failed to decode l1 origin"))?;

        let (parent_check, data) = take_data(data, 20);
        let (l1_origin_check, data) = take_data(data, 20);
        let (block_count, data) = unsigned_varint::decode::u64(data)
            .map_err(|_| alloy_rlp::Error::Custom("Failed to decode block count"))?;

        let (origin_bits, data) = decode_bitlist(data, block_count);
        let (block_tx_counts, data) = decode_block_tx_counts(data, block_count)
            .map_err(|_| alloy_rlp::Error::Custom("Failed to decode tx counts"))?;

        let total_txs = block_tx_counts.iter().sum();
        let (transactions, _) = decode_transactions(chain_id, data, total_txs)
            .map_err(|_| alloy_rlp::Error::Custom("Failed to decode transactions"))?;

        Ok(SpanBatch {
            rel_timestamp,
            l1_origin_num,
            parent_check: parent_check
                .try_into()
                .map_err(|_| alloy_rlp::Error::Custom("Failed to decode parent check"))?,
            l1_origin_check: l1_origin_check
                .try_into()
                .map_err(|_| alloy_rlp::Error::Custom("Failed to decode l1 origin check"))?,
            block_count,
            block_tx_counts,
            origin_bits,
            transactions,
            l1_inclusion_block,
        })
    }

    /// Returns a [BlockInput] vector for this batch. Contains all L2 block in the batch.
    pub fn block_inputs(&self, config: &RollupConfig) -> Vec<BlockInput> {
        let init_epoch_num = self.l1_origin_num
            - self
                .origin_bits
                .iter()
                .map(|b| if *b { 1 } else { 0 })
                .sum::<u64>();

        let mut inputs = Vec::new();
        let mut epoch_num = init_epoch_num;
        let mut tx_index = 0usize;
        for i in 0..self.block_count as usize {
            if self.origin_bits[i] {
                epoch_num += 1;
            }

            let tx_end = self.block_tx_counts[i] as usize;
            let transactions = self.transactions[tx_index..tx_index + tx_end].to_vec();
            tx_index += self.block_tx_counts[i] as usize;

            let timestamp =
                self.rel_timestamp + config.genesis.timestamp + i as u64 * config.block_time;

            let block_input = BlockInput {
                timestamp,
                transactions,
                l1_inclusion_block: self.l1_inclusion_block,
            };

            inputs.push(block_input);
        }

        inputs
    }

    /// Returns the L1 origin number of the last L2 block in the batch
    pub fn start_epoch_num(&self) -> u64 {
        self.l1_origin_num
            - self
                .origin_bits
                .iter()
                .map(|b| if *b { 1 } else { 0 })
                .sum::<u64>()
            + if self.origin_bits[0] { 1 } else { 0 }
    }
}

/// Splits a byte slice at the specified index (length) into a tuple of 2 byte slices
fn take_data(data: &[u8], length: usize) -> (&[u8], &[u8]) {
    (&data[0..length], &data[length..])
}

/// Decodes a bitlist into boolean values and returns a tuple of booleans + the original bitlist.
fn decode_bitlist(data: &[u8], len: u64) -> (Vec<bool>, &[u8]) {
    let mut bitlist = Vec::new();

    let len_up = (len + 7) / 8;
    let (bytes, data) = take_data(data, len_up as usize);

    for byte in bytes.iter().rev() {
        for i in 0..8 {
            let bit = (byte >> i) & 1 == 1;
            bitlist.push(bit);
        }
    }

    let bitlist = bitlist[..len as usize].to_vec();

    (bitlist, data)
}

/// Decodes the number of transactions in the batch into a U64 vector
fn decode_block_tx_counts(data: &[u8], block_count: u64) -> Result<(Vec<u64>, &[u8])> {
    let mut tx_counts = Vec::new();
    let mut data_ref = data;
    for _ in 0..block_count {
        let (count, d) = unsigned_varint::decode::u64(data_ref).unwrap();
        data_ref = d;
        tx_counts.push(count);
    }

    Ok((tx_counts, data_ref))
}

/// Decodes transactions in a batch and returns a [RawTransaction] vector
fn decode_transactions(
    chain_id: u64,
    data: &[u8],
    tx_count: u64,
) -> Result<(Vec<RawTransaction>, &[u8])> {
    let (contract_creation_bits, data) = decode_bitlist(data, tx_count);
    let (y_parity_bits, data) = decode_bitlist(data, tx_count);
    let (signatures, data) = decode_signatures(data, tx_count);

    let tos_count = contract_creation_bits.iter().filter(|b| !**b).count() as u64;
    let (tos, data) = decode_tos(data, tos_count);

    let (tx_datas, data) = decode_tx_data(data, tx_count);
    let (tx_nonces, data) = decode_uvarint_list(data, tx_count);
    let (tx_gas_limits, data) = decode_uvarint_list(data, tx_count);

    let legacy_tx_count = tx_datas
        .iter()
        .filter(|tx| matches!(tx, TxData::Legacy { .. }))
        .count() as u64;

    let (protected_bits, data) = decode_bitlist(data, legacy_tx_count);

    let mut txs = Vec::new();
    let mut legacy_i = 0;
    let mut tos_i = 0;

    for i in 0..tx_count as usize {
        let mut encoder = BytesMut::new();

        match &tx_datas[i] {
            TxData::Legacy {
                value,
                gas_price,
                data,
            } => {
                chain_id.encode(&mut encoder);
                tx_nonces[i].encode(&mut encoder);
                gas_price.encode(&mut encoder);
                tx_gas_limits[i].encode(&mut encoder);

                if contract_creation_bits[i] {
                    "".encode(&mut encoder);
                } else {
                    tos[tos_i].encode(&mut encoder);
                    tos_i += 1;
                }

                value.encode(&mut encoder);
                data.encode(&mut encoder);

                let parity = if y_parity_bits[i] { 1 } else { 0 };
                let v = if protected_bits[legacy_i] {
                    chain_id * 2 + 35 + parity
                } else {
                    27 + parity
                };

                v.encode(&mut encoder);
                signatures[i].0.encode(&mut encoder);
                signatures[i].1.encode(&mut encoder);

                let raw_tx = RawTransaction(encoder.to_vec());
                txs.push(raw_tx);
                legacy_i += 1;
            }
            TxData::Type1 {
                value,
                gas_price,
                data,
                access_list,
            } => {
                chain_id.encode(&mut encoder);
                tx_nonces[i].encode(&mut encoder);
                gas_price.encode(&mut encoder);
                tx_gas_limits[i].encode(&mut encoder);

                if contract_creation_bits[i] {
                    "".encode(&mut encoder);
                } else {
                    tos[tos_i].encode(&mut encoder);
                    tos_i += 1;
                }

                value.encode(&mut encoder);
                data.encode(&mut encoder);
                access_list.encode(&mut encoder);

                let parity = if y_parity_bits[i] { 1u64 } else { 0u64 };
                parity.encode(&mut encoder);
                signatures[i].0.encode(&mut encoder);
                signatures[i].1.encode(&mut encoder);

                let mut raw = encoder.to_vec();
                raw.insert(0, 1);
                let raw_tx = RawTransaction(raw);
                txs.push(raw_tx);
            }
            TxData::Type2 {
                value,
                max_fee,
                max_priority_fee,
                data,
                access_list,
            } => {
                chain_id.encode(&mut encoder);
                tx_nonces[i].encode(&mut encoder);
                max_priority_fee.encode(&mut encoder);
                max_fee.encode(&mut encoder);
                tx_gas_limits[i].encode(&mut encoder);

                if contract_creation_bits[i] {
                    "".encode(&mut encoder);
                } else {
                    tos[tos_i].encode(&mut encoder);
                    tos_i += 1;
                }

                value.encode(&mut encoder);
                data.encode(&mut encoder);
                access_list.encode(&mut encoder);

                let parity = if y_parity_bits[i] { 1u64 } else { 0u64 };

                parity.encode(&mut encoder);
                signatures[i].0.encode(&mut encoder);
                signatures[i].1.encode(&mut encoder);

                let mut raw = encoder.to_vec();
                raw.insert(0, 2);

                let raw_tx = RawTransaction(raw);
                txs.push(raw_tx);
            }
        }
    }

    Ok((txs, data))
}

/// Decodes transaction nonces in the batch into a U64 vector
fn decode_uvarint_list(data: &[u8], count: u64) -> (Vec<u64>, &[u8]) {
    let mut list = Vec::new();
    let mut data_ref = data;

    for _ in 0..count {
        let (nonce, d) = unsigned_varint::decode::u64(data_ref).unwrap();
        data_ref = d;
        list.push(nonce);
    }

    (list, data_ref)
}

/// Decodes EIP-2718 `TransactionType` formatted transactions in the batch into a [TxData] vector
fn decode_tx_data(data: &[u8], tx_count: u64) -> (Vec<TxData>, &[u8]) {
    let mut data_ref = data;
    let mut tx_datas = Vec::new();

    for _ in 0..tx_count {
        let (next, data) = match data_ref[0] {
            1 => {
                let mut rlp = &data_ref[1..];

                let value = U256::decode(&mut rlp).unwrap();
                let gas_price = U256::decode(&mut rlp).unwrap();
                let data = Vec::<u8>::decode(&mut rlp).unwrap();
                let access_list = AccessList::decode(&mut rlp).unwrap();

                let next = rlp.len() + 1;
                let data = TxData::Type1 {
                    value,
                    gas_price,
                    data: data.into(),
                    access_list,
                };

                (next, data)
            }
            2 => {
                let mut rlp = &data_ref[1..];
                let value = U256::decode(&mut rlp).unwrap();
                let max_priority_fee = U256::decode(&mut rlp).unwrap();
                let max_fee = U256::decode(&mut rlp).unwrap();
                let data = Vec::<u8>::decode(&mut rlp).unwrap();
                let access_list = AccessList::decode(&mut rlp).unwrap();

                let next = rlp.len() + 1;
                let data = TxData::Type2 {
                    value,
                    max_fee,
                    max_priority_fee,
                    data: data.into(),
                    access_list,
                };

                (next, data)
            }
            _ => {
                let mut rlp = &data_ref[1..];
                let value = U256::decode(&mut rlp).unwrap();
                let gas_price = U256::decode(&mut rlp).unwrap();
                let data = Vec::<u8>::decode(&mut rlp).unwrap();

                let next = rlp.len();
                let data = TxData::Legacy {
                    value,
                    gas_price,
                    data: data.into(),
                };

                (next, data)
            }
        };

        tx_datas.push(data);
        data_ref = &data_ref[next..];
    }

    (tx_datas, data_ref)
}

/// The transaction type - Legacy, EIP-2930, or EIP-1559
#[derive(Debug)]
enum TxData {
    /// A legacy transaction type
    Legacy {
        /// Transaction value
        value: U256,
        /// Transaction gas price
        gas_price: U256,
        /// Transaction calldata
        data: Bytes,
    },
    /// An EIP-2930 transaction type
    Type1 {
        /// Transaction value
        value: U256,
        /// Transaction gas price
        gas_price: U256,
        /// Transaction calldata
        data: Bytes,
        /// Access list as specified in EIP-2930
        access_list: AccessList,
    },
    /// An EIP-1559 transaction type
    Type2 {
        /// Transaction value
        value: U256,
        /// Max fee per gas as specified in EIP-1559
        max_fee: U256,
        /// Max priority fee as specified in EIP-1559
        max_priority_fee: U256,
        /// Transaction calldata
        data: Bytes,
        /// Access list as specified in EIP-2930
        access_list: AccessList,
    },
}

/// Decodes transaction `To` fields in the batch into an [Address] vector
fn decode_tos(data: &[u8], count: u64) -> (Vec<Address>, &[u8]) {
    let mut data_ref = data;
    let mut tos = Vec::new();
    for _ in 0..count {
        let (addr, d) = decode_address(data_ref);
        tos.push(addr);
        data_ref = d;
    }

    (tos, data_ref)
}

/// Decodes arbitrary slice of bytes into an [Address]
fn decode_address(data: &[u8]) -> (Address, &[u8]) {
    let (address_bytes, data) = take_data(data, 20);
    let address = Address::from_slice(address_bytes);
    (address, data)
}

/// Decodes transaction `R` & `S` signature fields in the batch into a (U256, U256) vector
fn decode_signatures(data: &[u8], tx_count: u64) -> (Vec<(U256, U256)>, &[u8]) {
    let mut sigs = Vec::new();
    let mut data_ref = data;
    for _ in 0..tx_count {
        let (r, d) = decode_u256(data_ref);
        data_ref = d;

        let (s, d) = decode_u256(data_ref);
        data_ref = d;

        sigs.push((r, s));
    }

    (sigs, data_ref)
}

/// Decodes a U256 from an arbitrary slice of bytes
fn decode_u256(data: &[u8]) -> (U256, &[u8]) {
    let (bytes, data) = take_data(data, 32);
    let value = U256::from_be_slice(bytes);
    (value, data)
}
