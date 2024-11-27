//! Contains an online implementation of the `BlobProvider` trait.

use alloy_eips::eip4844::{Blob, BlobTransactionSidecarItem, IndexedBlobHash};
use alloy_rpc_types_beacon::sidecar::{BeaconBlobBundle, BlobData};
use async_trait::async_trait;
use kona_derive::{errors::BlobProviderError, traits::BlobProvider};
use op_alloy_protocol::BlockInfo;
use reqwest::Client;

/// The config spec engine api method.
const SPEC_METHOD: &str = "eth/v1/config/spec";

/// The beacon genesis engine api method.
const GENESIS_METHOD: &str = "eth/v1/beacon/genesis";

/// The blob sidecars engine api method prefix.
const SIDECARS_METHOD_PREFIX: &str = "eth/v1/beacon/blob_sidecars";

/// A reduced genesis data.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReducedGenesisData {
    /// The genesis time.
    #[serde(rename = "genesis_time")]
    #[serde(with = "alloy_serde::quantity")]
    pub genesis_time: u64,
}

/// An API genesis response.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct APIGenesisResponse {
    /// The data.
    pub data: ReducedGenesisData,
}

/// A reduced config data.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReducedConfigData {
    /// The seconds per slot.
    #[serde(rename = "SECONDS_PER_SLOT")]
    #[serde(with = "alloy_serde::quantity")]
    pub seconds_per_slot: u64,
}

/// An API config response.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct APIConfigResponse {
    /// The data.
    pub data: ReducedConfigData,
}

impl APIConfigResponse {
    /// Creates a new API config response.
    pub const fn new(seconds_per_slot: u64) -> Self {
        Self { data: ReducedConfigData { seconds_per_slot } }
    }
}

impl APIGenesisResponse {
    /// Creates a new API genesis response.
    pub const fn new(genesis_time: u64) -> Self {
        Self { data: ReducedGenesisData { genesis_time } }
    }
}

/// An online implementation of the [BlobProvider] trait.
#[derive(Debug, Clone)]
pub struct OnlineBlobProvider {
    /// The base url.
    base: String,
    /// The inner reqwest client.
    inner: Client,
    /// The genesis time.
    genesis_time: u64,
    /// The slot interval.
    slot_interval: u64,
}

impl OnlineBlobProvider {
    /// Creates a new instance of the [OnlineBlobProvider].
    ///
    /// The `genesis_time` and `slot_interval` arguments are _optional_ and the
    /// [OnlineBlobProvider] will attempt to load them dynamically at runtime if they are not
    /// provided.
    pub async fn new_http(base: String) -> Result<Self, BlobProviderError> {
        let inner = Client::new();
        let genesis = inner
            .get(format!("{}/{}", base, GENESIS_METHOD))
            .send()
            .await
            .map_err(|_| BlobProviderError::Backend("Failed to fetch genesis".to_string()))?;
        let genesis_time = genesis
            .json::<APIGenesisResponse>()
            .await
            .map_err(|e| BlobProviderError::Backend(e.to_string()))?
            .data
            .genesis_time;
        let spec = inner
            .get(format!("{}/{}", base, SPEC_METHOD))
            .send()
            .await
            .map_err(|_| BlobProviderError::Backend("Failed to fetch config".to_string()))?;
        let slot_interval = spec
            .json::<APIConfigResponse>()
            .await
            .map_err(|e| BlobProviderError::Backend(e.to_string()))?
            .data
            .seconds_per_slot;
        Ok(Self { base, inner, genesis_time, slot_interval })
    }

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    async fn beacon_blob_side_cars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobData>, reqwest::Error> {
        let raw_response = self
            .inner
            .get(format!("{}/{}/{}", self.base, SIDECARS_METHOD_PREFIX, slot))
            .send()
            .await?;
        let raw_response = raw_response.json::<BeaconBlobBundle>().await?;

        // Filter the sidecars by the hashes, in-order.
        let mut sidecars = Vec::with_capacity(hashes.len());
        hashes.iter().for_each(|hash| {
            if let Some(sidecar) =
                raw_response.data.iter().find(|sidecar| sidecar.index == hash.index)
            {
                sidecars.push(sidecar.clone());
            }
        });

        Ok(sidecars)
    }

    /// Fetches blob sidecars for the given slot and blob hashes.
    pub async fn fetch_sidecars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobData>, BlobProviderError> {
        self.beacon_blob_side_cars(slot, hashes)
            .await
            .map_err(|e| BlobProviderError::Backend(e.to_string()))
    }

    /// Computes the slot for the given timestamp.
    pub const fn slot(
        genesis: u64,
        slot_time: u64,
        timestamp: u64,
    ) -> Result<u64, BlobProviderError> {
        if timestamp < genesis {
            return Err(BlobProviderError::SlotDerivation);
        }
        Ok((timestamp - genesis) / slot_time)
    }

    /// Fetches blob sidecars for the given block reference and blob hashes.
    pub async fn fetch_filtered_sidecars(
        &self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobTransactionSidecarItem>, BlobProviderError> {
        if blob_hashes.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate the slot for the given timestamp.
        let slot = Self::slot(self.genesis_time, self.slot_interval, block_ref.timestamp)?;

        // Fetch blob sidecars for the slot using the given blob hashes.
        let sidecars = self.fetch_sidecars(slot, blob_hashes).await?;

        // Filter blob sidecars that match the indicies in the specified list.
        let blob_hash_indicies = blob_hashes.iter().map(|b| b.index).collect::<Vec<u64>>();
        let filtered = sidecars
            .into_iter()
            .filter(|s| blob_hash_indicies.contains(&s.index))
            .collect::<Vec<_>>();

        // Validate the correct number of blob sidecars were retrieved.
        if blob_hashes.len() != filtered.len() {
            return Err(BlobProviderError::SidecarLengthMismatch(blob_hashes.len(), filtered.len()));
        }

        Ok(filtered
            .into_iter()
            .map(|s| BlobTransactionSidecarItem {
                index: s.index,
                blob: s.blob,
                kzg_commitment: s.kzg_commitment,
                kzg_proof: s.kzg_proof,
            })
            .collect::<Vec<BlobTransactionSidecarItem>>())
    }
}

#[async_trait]
impl BlobProvider for OnlineBlobProvider {
    type Error = BlobProviderError;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. The blobs are validated for their index and hashes using the specified
    /// [IndexedBlobHash].
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Box<Blob>>, Self::Error> {
        // Fetch the blob sidecars for the given block reference and blob hashes.
        let sidecars = self.fetch_filtered_sidecars(block_ref, blob_hashes).await?;

        // Validate the blob sidecars straight away with the num hashes.
        let blobs = sidecars
            .into_iter()
            .enumerate()
            .map(|(i, sidecar)| {
                let hash = blob_hashes
                    .get(i)
                    .ok_or(BlobProviderError::Backend("Missing blob hash".to_string()))?;
                match sidecar.verify_blob(&alloy_eips::eip4844::IndexedBlobHash {
                    hash: hash.hash,
                    index: hash.index,
                }) {
                    Ok(_) => Ok(sidecar.blob),
                    Err(e) => Err(BlobProviderError::Backend(e.to_string())),
                }
            })
            .collect::<Result<Vec<Box<Blob>>, BlobProviderError>>()
            .map_err(|e| BlobProviderError::Backend(e.to_string()))?;
        Ok(blobs)
    }
}
