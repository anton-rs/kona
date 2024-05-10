#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

pub mod block;
pub use block::{Block, BlockID, BlockInfo, BlockKind, L2BlockInfo, OpBlock, Withdrawal};

pub mod block_info;
pub use block_info::{L1BlockInfoBedrock, L1BlockInfoEcotone, L1BlockInfoTx};

pub mod raw_tx;
pub use raw_tx::RawTransaction;

pub mod deposits;
pub use deposits::{
    decode_deposit, DepositError, DepositSourceDomain, DepositSourceDomainIdentifier,
    L1InfoDepositSource, UpgradeDepositSource, UserDepositSource, DEPOSIT_EVENT_ABI_HASH,
};

pub mod genesis;
pub use genesis::Genesis;

pub mod params;
pub use params::*;

pub mod payload;
pub use payload::{
    L2ExecutionPayload, L2ExecutionPayloadEnvelope, PAYLOAD_MEM_FIXED_COST, PAYLOAD_TX_MEM_OVERHEAD,
};

pub mod rollup_config;
pub use rollup_config::RollupConfig;

pub mod attributes;
pub use attributes::{L2AttributesWithParent, L2PayloadAttributes};

pub mod system_config;
pub use system_config::{SystemAccounts, SystemConfig, SystemConfigUpdateType};
