#![allow(dead_code)]
//! Contains an online implementation of the [BlobProvider] trait.

use crate::{
    traits::BlobProvider,
    types::{Blob, BlockInfo, IndexedBlobHash},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_provider::Provider;
use alloy_transport_http::Http;
use async_trait::async_trait;
use reqwest::Client;
use tracing::debug;
use core::fmt::Display;

/// The node version engine api method.
pub const VERSION_METHOD: &str = "eth/v1/node/version";

/// The config spec engine api method.
pub const SPEC_METHOD: &str = "eth/v1/config/spec";

/// The beacon genesis engine api method.
pub const GENESIS_METHOD: &str = "eth/v1/beacon/genesis";

/// The blob sidecars engine api method prefix.
pub const SIDECARS_METHOD_PREFIX: &str = "eth/v1/beacon/blob_sidecars/";

/// The [BeaconClient] is a thin wrapper around the Beacon API.
pub trait BeaconClient {
    /// Returns the node version.
    fn node_version(&self) -> anyhow::Result<String>;

    /// Returns the config spec.
    fn config_spec(&self) -> anyhow::Result<APIConfigResponse>;

    /// Returns the beacon genesis.
    fn beacon_genesis(&self) -> anyhow::Result<APIGenesisResponse>;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    fn beacon_blob_side_cars(
        &self,
        fetch_all_sidecars: bool,
        slot: u64,
        hashes: Vec<IndexedBlobHash>,
    ) -> anyhow::Result<APIGetBlobSidecarsResponse>;
}

/// Specifies the derivation of a slot from a timestamp.
pub trait SlotDerivation {
    /// Converts a timestamp to a slot number.
    fn slot(genesis: u64, slot_time: u64, timestamp: u64) -> Result<u64>;
}

/// An error returned by the [OnlineBlobProvider].
#[derive(Debug)]
pub enum OnlineBlobProviderError {
    /// The number of specified blob hashes did not match the number of returned sidecars.
    SidecarLengthMismatch(usize, usize),
    /// A custom [anyhow::Error] occurred.
    Custom(anyhow::Error),
}

impl PartialEq for OnlineBlobProviderError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SidecarLengthMismatch(a, b), Self::SidecarLengthMismatch(c, d)) => a == c && b == d,
            (Self::Custom(_), Self::Custom(_)) => true,
            _ => false,
        }
    }
}

impl Display for OnlineBlobProviderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SidecarLengthMismatch(a, b) => write!(f, "expected {} sidecars but got {}", a, b),
            Self::Custom(err) => write!(f, "{}", err),
        }
    }
}

impl From<anyhow::Error> for OnlineBlobProviderError {
    fn from(err: anyhow::Error) -> Self {
        Self::Custom(err)
    }
}

/// An online implementation of the [BlobProvider] trait.
#[derive(Debug)]
pub struct OnlineBlobProvider<T: Provider<Http<Client>>, B: BeaconClient, S: SlotDerivation> {
    /// The inner Ethereum JSON-RPC provider.
    inner: T,
    /// The Beacon API client.
    beacon_client: B,
    /// Beacon Genesis used for the time to slot conversion.
    genesis: Option<BeaconGenesis>,
    /// Config spec used for the time to slot conversion.
    config_spec: Option<ConfigSpec>,
    /// Phantom data for slot derivation.
    _slot_derivation: PhantomData<S>,
}

impl<T: Provider<Http<Client>>, B: BeaconClient, S: SlotDerivation> OnlineBlobProvider<T, B, S> {
    /// Creates a new instance of the [OnlineBlobProvider].
    ///
    /// The `genesis` and `config_spec` arguments are _optional_ and the [OnlineBlockProvider]
    /// will attempt to load them dynamically at runtime if they are not provided.
    pub fn new(inner: T, beacon_client: B, genesis: Option<BeasonGenesis>, config_spec: Option<ConfigSpec>) -> Self {
        Self { inner, beacon_client, genesis, config_spec }
    }

    /// Loads the beacon genesis and config spec 
    pub fn load_configs(&mut self) -> Result<(), OnlineBlobProviderError> {
        if self.genesis.is_none() {
            debug!("Loading missing BeaconGenesis");
            self.genesis = Some(self.beacon_client.beacon_genesis()?);
        }
        if self.config_spec.is_none() {
            debug!("Loading missing ConfigSpec");
            self.config_spec = Some(self.beacon_client.config_spec()?);
        }
        Ok(())
    }

    /// Fetches blob sidecars for the given slot and blob hashes.
    pub async fn fetch_sidecars(&self, slot: u64, hashes: Vec<IndexedBlobHash>) -> Result<APIBlobSidecar, OnlineBlobProviderError> {
        unimplemented!("fetching blob sidecars is not implemented");
    }


    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    pub async fn get_blob_sidecars(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: Vec<IndexedBlobHash>,
    ) -> Result<Vec<BlobSidecar>, OnlineBlobProviderError> {
        if blob_hashes.is_empty() {
            return Ok(Vec::new());
        }

        // Fetches [BeaconGenesis] and [ConfigSpec] configs if not previously loaded.
        self.load_configs()?;

        // Extract the genesis timestamp and slot interval from the loaded configs.
        let genesis = self.genesis.expect("Genesis Config Loaded").data.genesis_time;
        let interval = self.config_spec.expect("Config Spec Loaded").data.seconds_per_slot;

        // Calculate the slot for the given timestamp.
        let slot = S::slot(genesis, interval, block_ref.timestamp)?;

        // Fetch blob sidecars for the slot using the given blob hashes.
        let sidecars = self.fetch_sidecars(slot, blob_hashes).await?;

        // Filter blob sidecars that match the indicies in the specified list.
        let blob_hash_indicies = blob_hashes.iter().map(|b| b.index).collect::<Vec<_>>();
        let filtered = sidecars.iter().filter(|s| blob_hashes.contains(s.index)).collect::<Vec<_>>();

        // Validate the correct number of blob sidecars were retrieved.
        if blob_hashes.len() != filtered.len() {
            return Err(OnlineBlobProviderError::SidecarLengthMismatch(blob_hashes.len(), filtered.len()));
        }

        Ok(filtered.iter().map(|s| s.blob_sidecar()).collect::<Vec<BlobSidecar>>())
    }
}

/// Constructs a list of [Blob]s from [BlobSidecar]s and the specified [IndexedBlobHash]es.
pub fn blobs_from_sidecars(sidecars: &[BlobSidecar], hashes: &[IndexedBlobHash]) -> anyhow::Result<Vec<Bob>> {
    if sidecars.len() != hashes.len() {
        return Err(anyhow::anyhow!("blob sidecars and hashes length mismatch, {} != {}", sidecars.len(), hashes.len()));
    }

    let mut blobs = Vec::with_capacity(sidecars.len());
    for (i, sidecar) in sidecars.iter().enumerate() {
        let hash =  hashes.get(i).ok_or(anyhow::anyhow!("failed to get blob hash"))?;
        if sidecar.index != hash.index {
            return Err(anyhow::anyhow!("invalid sidecar ordering, blob hash index {} does not match sidecar index {}", hash.index, sidecar.index));
        }

        // Ensure the blob's kzg commitment hashes to the expected value.

		// hash := eth.KZGToVersionedHash(kzg4844.Commitment(sidecar.KZGCommitment))
		// if hash != ih.Hash {
		// 	return nil, fmt.Errorf("expected hash %s for blob at index %d but got %s", ih.Hash, ih.Index, hash)
		// }

        // Confirm blob data is valid by verifying its proof against the commitment

		// if err := eth.VerifyBlobProof(&sidecar.Blob, kzg4844.Commitment(sidecar.KZGCommitment), kzg4844.Proof(sidecar.KZGProof)); err != nil {
		// 	return nil, fmt.Errorf("blob at index %d failed verification: %w", i, err)
		// }

        blobs.push(sidecar.blob);
    }
    Ok(blobs)
}

#[async_trait]
impl<T: Provider<Http<Client>>> BlobProvider for OnlineBlobProvider<T> {
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: Vec<IndexedBlobHash>,
    ) -> Result<Vec<Blob>> {
        let sidecars = self.get_blob_sidecars(block_ref, blob_hashes).await.map_err(|e| anyhow::anyhow!(e))?;
        blobs_from_sidecars(sidecars, blob_hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::online::test_utils::spawn_anvil;

    #[tokio::test]
    async fn test_get_blob_sidecars_empty_hashes() {
        let (provider, _anvil) = spawn_anvil();
        let blob_provider = OnlineBlobProvider::new(provider);
        let block_ref = BlockInfo::default();
        let blob_hashes = Vec::new();
        let result = blob_provider.get_blob_sidecars(&block_ref, blob_hashes).await;
        assert!(result.unwrap().is_empty());
    }
}
