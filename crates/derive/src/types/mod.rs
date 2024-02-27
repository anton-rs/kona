//! This module contains all of the types used within the derivation pipeline.

use alloc::vec::Vec;
use alloy_rlp::{Decodable, Encodable};

mod batch;
pub use batch::Batch;

mod batch_type;
pub use batch_type::BatchType;

mod system_config;
pub use system_config::{
    SystemAccounts, SystemConfig, SystemConfigUpdateType, CONFIG_UPDATE_EVENT_VERSION_0,
    CONFIG_UPDATE_TOPIC,
};

mod rollup_config;
pub use rollup_config::RollupConfig;

pub mod spans;
pub use spans::{SpanBatch, SpanBatchBuilder, SpanBatchElement, SPAN_BATCH_TYPE};

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

mod frame;
pub use frame::Frame;

mod channel;
pub use channel::Channel;

mod errors;
pub use errors::{DecodeError, StageError, StageResult};

mod single_batch;
pub use single_batch::SingleBatch;

/// A raw transaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTransaction(pub Vec<u8>);

impl Encodable for RawTransaction {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.0.encode(out)
    }
}

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
