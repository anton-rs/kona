//! Contains sidecar types for blobs.

use alloc::{string::String, vec::Vec};
use alloy_eips::eip4844::Blob;
use alloy_primitives::FixedBytes;

#[cfg(feature = "online")]
use crate::types::IndexedBlobHash;
#[cfg(feature = "online")]
use alloy_primitives::B256;
#[cfg(feature = "online")]
use c_kzg::{Bytes48, KzgProof, KzgSettings};
#[cfg(feature = "online")]
use revm::primitives::kzg::{G1_POINTS, G2_POINTS};
#[cfg(feature = "online")]
use sha2::{Digest, Sha256};
#[cfg(feature = "online")]
use tracing::warn;

#[cfg(feature = "serde")]
use serde::de::Deserialize;

#[cfg(feature = "serde")]
use core::str::FromStr;

/// KZG Proof Size
pub const KZG_PROOF_SIZE: usize = 48;

/// KZG Commitment Size
pub const KZG_COMMITMENT_SIZE: usize = 48;

/// The versioned hash version for KZG.
#[cfg(feature = "online")]
pub(crate) const VERSIONED_HASH_VERSION_KZG: u8 = 0x01;

#[cfg(feature = "serde")]
fn parse_u64_string<'de, T, D>(de: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: core::fmt::Display,
{
    String::deserialize(de)?.parse().map_err(serde::de::Error::custom)
}

/// A blob sidecar.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobSidecar {
    /// The blob.
    pub blob: Blob,
    /// The index.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "parse_u64_string"))]
    pub index: u64,
    /// The KZG commitment.
    #[cfg_attr(feature = "serde", serde(rename = "kzg_commitment"))]
    pub kzg_commitment: FixedBytes<KZG_COMMITMENT_SIZE>,
    /// The KZG proof.
    #[cfg_attr(feature = "serde", serde(rename = "kzg_proof"))]
    pub kzg_proof: FixedBytes<KZG_PROOF_SIZE>,
}

impl BlobSidecar {
    /// Verify the blob sidecar against it's [IndexedBlobHash].
    #[cfg(feature = "online")]
    pub fn verify_blob(&self, hash: &IndexedBlobHash) -> anyhow::Result<()> {
        if self.index as usize != hash.index {
            return Err(anyhow::anyhow!(
                "invalid sidecar ordering, blob hash index {} does not match sidecar index {}",
                hash.index,
                self.index
            ));
        }

        // Ensure the blob's kzg commitment hashes to the expected value.
        if self.to_kzg_versioned_hash() != hash.hash {
            return Err(anyhow::anyhow!(
                "expected hash {} for blob at index {} but got {}",
                hash.hash,
                hash.index,
                B256::from(self.to_kzg_versioned_hash())
            ));
        }

        // Confirm blob data is valid by verifying its proof against the commitment
        match self.verify_blob_kzg_proof() {
            Ok(true) => Ok(()),
            Ok(false) => Err(anyhow::anyhow!("blob at index {} failed verification", self.index)),
            Err(e) => {
                Err(anyhow::anyhow!("blob at index {} failed verification: {}", self.index, e))
            }
        }
    }

    /// Verifies the blob kzg proof.
    #[cfg(feature = "online")]
    pub fn verify_blob_kzg_proof(&self) -> anyhow::Result<bool> {
        let how = |e: c_kzg::Error| anyhow::anyhow!(e);
        let blob = c_kzg::Blob::from_bytes(self.blob.as_slice()).map_err(how)?;
        let commitment = Bytes48::from_bytes(self.kzg_commitment.as_slice()).map_err(how)?;
        let kzg_proof = Bytes48::from_bytes(self.kzg_proof.as_slice()).map_err(how)?;
        let settings = KzgSettings::load_trusted_setup(&G1_POINTS.0, &G2_POINTS.0).map_err(how)?;
        match KzgProof::verify_blob_kzg_proof(&blob, &commitment, &kzg_proof, &settings) {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("Failed to verify blob KZG proof: {:?}", e);
                Ok(false)
            }
        }
    }

    /// `VERSIONED_HASH_VERSION_KZG ++ sha256(commitment)[1..]`
    #[cfg(feature = "online")]
    pub fn to_kzg_versioned_hash(&self) -> [u8; 32] {
        let commitment = self.kzg_commitment.as_slice();
        let mut hash: [u8; 32] = Sha256::digest(commitment).into();
        hash[0] = VERSIONED_HASH_VERSION_KZG;
        hash
    }
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
    #[cfg_attr(feature = "serde", serde(deserialize_with = "parse_u64_string"))]
    pub slot: u64,
    /// The proposer index.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "parse_u64_string"))]
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
    #[cfg_attr(feature = "serde", serde(deserialize_with = "parse_u64_string"))]
    pub genesis_time: u64,
}

/// An API genesis response.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct APIGenesisResponse {
    /// The data.
    pub data: ReducedGenesisData,
}

impl APIGenesisResponse {
    /// Creates a new API genesis response.
    pub fn new(genesis_time: u64) -> Self {
        Self { data: ReducedGenesisData { genesis_time } }
    }
}

/// A reduced config data.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReducedConfigData {
    /// The seconds per slot.
    #[cfg_attr(feature = "serde", serde(rename = "SECONDS_PER_SLOT"))]
    #[cfg_attr(feature = "serde", serde(deserialize_with = "parse_u64_string"))]
    pub seconds_per_slot: u64,
}

/// An API config response.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct APIConfigResponse {
    /// The data.
    pub data: ReducedConfigData,
}

impl APIConfigResponse {
    /// Creates a new API config response.
    pub fn new(seconds_per_slot: u64) -> Self {
        Self { data: ReducedConfigData { seconds_per_slot } }
    }
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
