//! This module contains all of the types used within the derivation pipeline.

mod system_config;
pub use system_config::{SystemAccounts, SystemConfig};

mod rollup_config;
pub use rollup_config::RollupConfig;

mod transaction;
pub use transaction::{TxDeposit, TxEip1559, TxEip2930, TxEip4844, TxEnvelope, TxLegacy, TxType};

mod network;
pub use network::{Receipt as NetworkReceipt, Sealable, Sealed, Transaction, TxKind};

mod header;
pub use header::{Header, EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};

mod block;
pub use block::{BlockId, BlockInfo, BlockKind};

mod receipt;
pub use receipt::{Receipt, ReceiptWithBloom};

mod eips;
pub use eips::{
    calc_blob_gasprice, calc_excess_blob_gas, calc_next_block_base_fee, eip1559, eip2718, eip2930,
    eip4788, eip4844,
};

mod genesis;
pub use genesis::Genesis;

use alloc::string::String;
use alloc::vec::Vec;
use alloy_primitives::{hex, Address, BlockHash};
use alloy_rlp::Decodable;

mod single_batch;
pub use single_batch::SingleBatch;

mod span_batch;
pub use span_batch::SpanBatch;

/// A raw transaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTransaction(pub Vec<u8>);

impl Decodable for RawTransaction {
    /// Decodes RLP encoded bytes into [RawTransaction] bytes
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let tx_bytes: Vec<u8> = Decodable::decode(buf)?;
        Ok(Self(tx_bytes))
    }
}

/// A single L2 block derived from a batch.
#[derive(Debug, Clone)]
pub struct BlockInput {
    /// Timestamp of the L2 block
    pub timestamp: u64,
    /// Transactions included in this block
    pub transactions: Vec<RawTransaction>,
    /// The L1 block this batch was fully derived from
    pub l1_inclusion_block: u64,
}
