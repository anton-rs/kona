#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

// Re-export `op-alloy-consensus` types.
pub use op_alloy_consensus::Hardforks;

// Re-export `op-alloy-protocol` types.
pub use op_alloy_protocol::{
    starts_with_2781_deposit, BlockInfo, Channel, ChannelId, Frame, L2BlockInfo, CHANNEL_ID_LENGTH,
    DERIVATION_VERSION_0, FJORD_MAX_RLP_BYTES_PER_CHANNEL, FRAME_OVERHEAD, MAX_FRAME_LEN,
    MAX_RLP_BYTES_PER_CHANNEL,
};

// Re-export `superchain-primitives` types.
pub use superchain_primitives::*;

// Re-export `alloy-primitives`.
pub use alloy_primitives;

// Re-export `alloy-consensus` types.
pub use alloy_consensus::{
    Header, Receipt, Signed, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant,
    TxEip4844WithSidecar, TxEnvelope, TxLegacy,
};

// Re-export `alloy-eips` eip4844 types.
pub use alloy_eips::eip4844::{Blob, BYTES_PER_BLOB, VERSIONED_HASH_VERSION_KZG};

/// Re-export the [Withdrawal] type from the [alloy_eips] crate.
///
/// [Withdrawal]: alloy_eips::eip4895::Withdrawal
pub use alloy_eips::eip4895::Withdrawal;

pub mod block;
pub use block::{Block, BlockKind, OpBlock};

pub mod block_info;
pub use block_info::{L1BlockInfoBedrock, L1BlockInfoEcotone, L1BlockInfoTx};

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

pub mod blob;
pub use blob::{BlobData, BlobDecodingError, IndexedBlobHash};

pub mod sidecar;
pub use sidecar::{
    APIBlobSidecar, APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse,
    APIVersionResponse, BeaconBlockHeader, BlobSidecar, SignedBeaconBlockHeader,
    VersionInformation, KZG_COMMITMENT_SIZE, KZG_PROOF_SIZE,
};
