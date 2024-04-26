#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub mod attributes;
pub mod batch;
pub mod blob;
pub mod block;
pub mod block_info;
pub mod channel;
pub mod deposits;
pub mod ecotone;
pub mod frame;
pub mod genesis;
pub mod params;
pub mod payload;
pub mod raw_tx;
pub mod rollup_config;
pub mod sidecar;
pub mod system_config;

/// The prelude exports common types and traits.
pub mod prelude {
    pub use crate::{
        attributes::{L2AttributesWithParent, L2PayloadAttributes},
        batch::{
            Batch, BatchType, BatchValidity, BatchWithInclusionBlock, RawSpanBatch, SingleBatch,
            SpanBatch, SpanBatchBits, SpanBatchEip1559TransactionData,
            SpanBatchEip2930TransactionData, SpanBatchElement, SpanBatchError,
            SpanBatchLegacyTransactionData, SpanBatchPayload, SpanBatchPrefix,
            SpanBatchTransactionData, SpanBatchTransactions, SpanDecodingError,
            MAX_SPAN_BATCH_SIZE,
        },
        blob::{Blob, BlobData, BlobDecodingError, IndexedBlobHash},
        block::{Block, BlockID, BlockInfo, BlockKind, L2BlockInfo, OpBlock, Withdrawal},
        block_info::{L1BlockInfoBedrock, L1BlockInfoEcotone, L1BlockInfoTx},
        channel::{Channel, ChannelID, CHANNEL_ID_LENGTH, MAX_RLP_BYTES_PER_CHANNEL},
        deposits::{
            DepositError, DepositSourceDomain, DepositSourceDomainIdentifier, L1InfoDepositSource,
            UpgradeDepositSource, UserDepositSource, DEPOSIT_EVENT_ABI, DEPOSIT_EVENT_ABI_HASH,
            DEPOSIT_EVENT_VERSION_0,
        },
        ecotone::*,
        frame::{Frame, FRAME_OVERHEAD, MAX_FRAME_LEN},
        genesis::Genesis,
        payload::{
            L2ExecutionPayload, L2ExecutionPayloadEnvelope, PAYLOAD_MEM_FIXED_COST,
            PAYLOAD_TX_MEM_OVERHEAD,
        },
        raw_tx::RawTransaction,
        rollup_config::RollupConfig,
        sidecar::{
            APIBlobSidecar, APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse,
            APIVersionResponse, BeaconBlockHeader, BlobSidecar, SignedBeaconBlockHeader,
            VersionInformation, KZG_COMMITMENT_SIZE, KZG_PROOF_SIZE,
        },
        system_config::{SystemAccounts, SystemConfig, SystemConfigUpdateType},
    };
}
