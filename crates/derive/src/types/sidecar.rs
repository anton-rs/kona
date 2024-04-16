//! Contains sidecar types for blobs.

use crate::types::Blob;
use alloc::{string::String, vec::Vec};
use alloy_primitives::FixedBytes;

/// KZG Proof Size
pub const KZG_PROOF_SIZE: usize = 48;

/// KZG Commitment Size
pub const KZG_COMMITMENT_SIZE: usize = 48;

/// A blob sidecar.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobSidecar {
    /// The blob.
    pub blob: Blob,
    /// The index.
    pub index: u64,
    /// The KZG commitment.
    #[cfg_attr(feature = "serde", serde(rename = "kzg_commitment"))]
    pub kzg_commitment: FixedBytes<KZG_COMMITMENT_SIZE>,
    /// The KZG proof.
    #[cfg_attr(feature = "serde", serde(rename = "kzg_proof"))]
    pub kzg_proof: FixedBytes<KZG_PROOF_SIZE>,
}

/// An API blob sidecar.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct APIBlobSidecar {
    /// The inner blob sidecar.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: BlobSidecar,
    /// The signed block header.
    #[cfg_attr(feature = "serde", serde(rename = "signed_block_header"))]
    pub signed_block_header: SignedBeaconBlockHeader,
    // The inclusion-proof of the blob-sidecar into the beacon-block is ignored,
    // since we verify blobs by their versioned hashes against the execution-layer block instead.
}

/// A signed beacon block header.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedBeaconBlockHeader {
    /// The message.
    pub message: BeaconBlockHeader,
    // The signature is ignored, since we verify blobs against EL versioned-hashes
}

/// A beacon block header.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BeaconBlockHeader {
    /// The slot.
    pub slot: u64,
    /// The proposer index.
    pub proposer_index: u64,
    /// The parent root.
    #[cfg_attr(feature = "serde", serde(rename = "parent_root"))]
    pub parent_root: FixedBytes<32>,
    /// The state root.
    #[cfg_attr(feature = "serde", serde(rename = "state_root"))]
    pub state_root: FixedBytes<32>,
    /// The body root.
    #[cfg_attr(feature = "serde", serde(rename = "body_root"))]
    pub body_root: FixedBytes<32>,
}

/// An response for fetching blob sidecars.
#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct APIGetBlobSidecarsResponse {
    /// The data.
    pub data: Vec<APIBlobSidecar>,
}

impl Clone for APIGetBlobSidecarsResponse {
    fn clone(&self) -> Self {
        let mut data = Vec::with_capacity(self.data.len());
        for sidecar in &self.data {
            data.push(sidecar.clone());
        }
        Self { data }
    }
}

/// A reduced genesis data.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReducedGenesisData {
    /// The genesis time.
    #[cfg_attr(feature = "serde", serde(rename = "genesis_time"))]
    pub genesis_time: u64,
}

/// An API genesis response.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct APIGenesisResponse {
    /// The data.
    pub data: ReducedGenesisData,
}

/// A reduced config data.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReducedConfigData {
    /// The seconds per slot.
    #[cfg_attr(feature = "serde", serde(rename = "SECONDS_PER_SLOT"))]
    pub seconds_per_slot: u64,
}

/// An API config response.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct APIConfigResponse {
    /// The data.
    pub data: ReducedConfigData,
}

/// An API version response.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct APIVersionResponse {
    /// The data.
    pub data: VersionInformation,
}

/// Version information.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VersionInformation {
    /// The version.
    pub version: String,
}