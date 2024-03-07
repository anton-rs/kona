//! Span Batch Transactions

use crate::types::spans::{SpanBatchBits, SpanBatchSignature};
use alloy_primitives::{Bytes, Address, U64};
use alloy_rlp::Decodable;
use alloc::vec::Vec;

/// Transactions in a span batch
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanBatchTransactions {
    /// The total block transaction count
    pub total_block_tx_count: U64,

    // 8 fields
    /// The contract creation bits
    pub contract_creation_bits: SpanBatchBits,
    /// The y parity bits
    pub y_parity_bits: SpanBatchBits,
    /// The transaction signatures
    pub tx_sigs: Vec<SpanBatchSignature>,

    /// Transaction nonces
    pub tx_nonces: Vec<U64>,
    /// Transaction gases
    pub tx_gases: Vec<U64>,
    /// Transaction to addresses
    pub tx_tos: Vec<Address>,
    /// Transaction data
    pub tx_datas: Vec<Bytes>,
    /// The protected bits
    pub protected_bits: Bytes,

    // Intermediate variables which can be recovered

    /// The transaction types
    pub tx_types: Vec<i32>,
    /// The total legacy transaction count
    pub total_legacy_tx_count: U64,
}

impl Decodable for SpanBatchTransactions {
    fn decode(_r: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let transactions = SpanBatchTransactions::default();
        // transactions
        //     .decode_total_block_tx_count(r)
        //     .map_err(|_| alloy_rlp::Error::Custom("Decoding total block tx count failed"))?;
        Ok(transactions)
    }
}
