#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

// Re-export superchain-primitives
pub use superchain_primitives::*;

// Re-export superchain bindings if the `serde` feature is enabled
#[cfg(feature = "serde")]
pub use superchain_registry::*;

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

/// Retrieves the [RollupConfig] for the given `chain_id`.
pub fn get_rollup_config(chain_id: u64) -> anyhow::Result<RollupConfig> {
    if cfg!(feature = "serde") {
        superchain_registry::ROLLUP_CONFIGS
            .get(&chain_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown chain ID: {}", chain_id))
            .cloned()
    } else {
        superchain_primitives::rollup_config_from_chain_id(chain_id)
    }
}
