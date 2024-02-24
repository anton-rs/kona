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
#[derive(Clone, PartialEq, Eq)]
pub struct RawTransaction(pub Vec<u8>);

impl Decodable for RawTransaction {
    /// Decodes RLP encoded bytes into [RawTransaction] bytes
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let tx_bytes: Vec<u8> = Decodable::decode(buf)?;
        Ok(Self(tx_bytes))
    }
}

/// A single L2 block derived from a batch.
#[derive(Clone)]
pub struct BlockInput {
    /// Timestamp of the L2 block
    pub timestamp: u64,
    /// Transactions included in this block
    pub transactions: Vec<RawTransaction>,
    /// The L1 block this batch was fully derived from
    pub l1_inclusion_block: u64,
}

/// The global `Magi` configuration.
#[derive(Clone)]
pub struct Config {
    /// The L1 chain RPC URL
    pub l1_rpc_url: String,
    /// The L2 chain RPC URL
    pub l2_rpc_url: String,
    /// The L2 engine API URL
    pub l2_engine_url: String,
    /// The L2 chain config
    pub chain: ChainConfig,
    /// Engine API JWT Secret.
    /// This is used to authenticate with the engine API
    pub jwt_secret: String,
    /// A trusted L2 RPC URL to use for fast/checkpoint syncing
    pub checkpoint_sync_url: Option<String>,
    /// The port of the `Magi` RPC server
    pub rpc_port: u16,
    /// If devnet is enabled.
    pub devnet: bool,
}

/// Configurations for a blockchain.
#[derive(Clone)]
pub struct ChainConfig {
    /// The network name
    pub network: String,
    /// The L1 chain id
    pub l1_chain_id: u64,
    /// The L2 chain id
    pub l2_chain_id: u64,
    /*
    /// The L1 genesis block referenced by the L2 chain
    pub l1_start_epoch: Epoch,
    */
    /// The L2 genesis block info
    pub l2_genesis: BlockInfo,
    /*
    /// The initial system config value
    pub system_config: SystemConfig,
    */
    /// The batch inbox address
    pub batch_inbox: Address,
    /// The deposit contract address
    pub deposit_contract: Address,
    /// The L1 system config contract address
    pub system_config_contract: Address,
    /// The maximum byte size of all pending channels
    pub max_channel_size: u64,
    /// The max timeout for a channel (as measured by the frame L1 block number)
    pub channel_timeout: u64,
    /// Number of L1 blocks in a sequence window
    pub seq_window_size: u64,
    /// Maximum timestamp drift
    pub max_seq_drift: u64,
    /// Timestamp of the regolith hardfork
    pub regolith_time: u64,
    /// Timestamp of the canyon hardfork
    pub canyon_time: u64,
    /// Timestamp of the delta hardfork
    pub delta_time: u64,
    /// Network blocktime
    pub blocktime: u64,
}
