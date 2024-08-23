#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![no_std]

// Re-export superchain-primitives.
pub use superchain_primitives::*;

// Re-export alloy-primitives.
pub use alloy_primitives;

extern crate alloc;

/// Re-export the [Withdrawal] type from the [alloy_eips] crate.
///
/// [Withdrawal]: alloy_eips::eip4895::Withdrawal
pub use alloy_eips::eip4895::Withdrawal;

pub mod block;
pub use block::{Block, BlockInfo, BlockKind, L2BlockInfo, OpBlock};

pub mod block_info;
pub use block_info::{L1BlockInfoBedrock, L1BlockInfoEcotone, L1BlockInfoTx};

pub mod raw_tx;
pub use raw_tx::RawTransaction;

pub mod deposits;
pub use deposits::{
    decode_deposit, DepositError, DepositSourceDomain, DepositSourceDomainIdentifier,
    L1InfoDepositSource, UpgradeDepositSource, UserDepositSource, DEPOSIT_EVENT_ABI_HASH,
};

pub mod payload;
pub use payload::{
    L2ExecutionPayload, L2ExecutionPayloadEnvelope, PAYLOAD_MEM_FIXED_COST, PAYLOAD_TX_MEM_OVERHEAD,
};

pub mod attributes;
pub use attributes::{L2AttributesWithParent, L2PayloadAttributes};
