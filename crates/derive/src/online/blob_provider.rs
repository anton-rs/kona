//! Contains an online implementation of the [BlobProvider] trait.

use crate::{
    online::BeaconClient,
    traits::BlobProvider,
    types::{APIBlobSidecar, Blob, BlobSidecar, BlockInfo, IndexedBlobHash},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_provider::Provider;
use alloy_transport_http::Http;
use async_trait::async_trait;
use core::{fmt::Display, marker::PhantomData};
use reqwest::Client;
use tracing::debug;

/// Specifies the derivation of a slot from a timestamp.
pub trait SlotDerivation {
    /// Converts a timestamp to a slot number.
    fn slot(genesis: u64, slot_time: u64, timestamp: u64) -> anyhow::Result<u64>;
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
            (Self::SidecarLengthMismatch(a, b), Self::SidecarLengthMismatch(c, d)) => {
                a == c && b == d
            }
            (Self::Custom(_), Self::Custom(_)) => true,
            _ => false,
        }
    }
}

impl Display for OnlineBlobProviderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
    _inner: T,
    /// Whether to fetch all sidecars.
    fetch_all_sidecars: bool,
    /// The Beacon API client.
    beacon_client: B,
    /// Beacon Genesis time used for the time to slot conversion.
    genesis_time: Option<u64>,
    /// Slot interval used for the time to slot conversion.
    slot_interval: Option<u64>,
    /// Phantom data for slot derivation.
    _slot_derivation: PhantomData<S>,
}

impl<T: Provider<Http<Client>>, B: BeaconClient, S: SlotDerivation> OnlineBlobProvider<T, B, S> {
    /// Creates a new instance of the [OnlineBlobProvider].
    ///
    /// The `genesis_time` and `slot_interval` arguments are _optional_ and the
    /// [OnlineBlockProvider] will attempt to load them dynamically at runtime if they are not
    /// provided.
    pub fn new(
        _inner: T,
        fetch_all_sidecars: bool,
        beacon_client: B,
        genesis_time: Option<u64>,
        slot_interval: Option<u64>,
    ) -> Self {
        Self {
            _inner,
            fetch_all_sidecars,
            beacon_client,
            genesis_time,
            slot_interval,
            _slot_derivation: PhantomData,
        }
    }

    /// Loads the beacon genesis and config spec
    pub async fn load_configs(&mut self) -> Result<(), OnlineBlobProviderError> {
        if self.genesis_time.is_none() {
            debug!("Loading missing BeaconGenesis");
            self.genesis_time = Some(self.beacon_client.beacon_genesis().await?.data.genesis_time);
        }
        if self.slot_interval.is_none() {
            debug!("Loading missing ConfigSpec");
            self.slot_interval =
                Some(self.beacon_client.config_spec().await?.data.seconds_per_slot);
        }
        Ok(())
    }

    /// Fetches blob sidecars for the given slot and blob hashes.
    pub async fn fetch_sidecars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<APIBlobSidecar>, OnlineBlobProviderError> {
        self.beacon_client
            .beacon_blob_side_cars(self.fetch_all_sidecars, slot, hashes)
            .await
            .map(|r| r.data)
            .map_err(|e| e.into())
    }

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    pub async fn get_blob_sidecars(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobSidecar>, OnlineBlobProviderError> {
        if blob_hashes.is_empty() {
            return Ok(Vec::new());
        }

        // Fetches the genesis timestamp and slot interval from the
        // [BeaconGenesis] and [ConfigSpec] if not previously loaded.
        self.load_configs().await?;

        // Extract the genesis timestamp and slot interval from the loaded configs.
        let genesis = self.genesis_time.expect("Genesis Config Loaded");
        let interval = self.slot_interval.expect("Config Spec Loaded");

        // Calculate the slot for the given timestamp.
        let slot = S::slot(genesis, interval, block_ref.timestamp)?;

        // Fetch blob sidecars for the slot using the given blob hashes.
        let sidecars = self.fetch_sidecars(slot, blob_hashes).await?;

        // Filter blob sidecars that match the indicies in the specified list.
        let blob_hash_indicies = blob_hashes.iter().map(|b| b.index).collect::<Vec<_>>();
        let filtered = sidecars
            .into_iter()
            .filter(|s| blob_hash_indicies.contains(&(s.inner.index as usize)))
            .collect::<Vec<_>>();

        // Validate the correct number of blob sidecars were retrieved.
        if blob_hashes.len() != filtered.len() {
            return Err(OnlineBlobProviderError::SidecarLengthMismatch(
                blob_hashes.len(),
                filtered.len(),
            ));
        }

        Ok(filtered.into_iter().map(|s| s.inner).collect::<Vec<BlobSidecar>>())
    }
}

/// Constructs a list of [Blob]s from [BlobSidecar]s and the specified [IndexedBlobHash]es.
pub(crate) fn blobs_from_sidecars(
    sidecars: &[BlobSidecar],
    hashes: &[IndexedBlobHash],
) -> anyhow::Result<Vec<Blob>> {
    if sidecars.len() != hashes.len() {
        return Err(anyhow::anyhow!(
            "blob sidecars and hashes length mismatch, {} != {}",
            sidecars.len(),
            hashes.len()
        ));
    }

    let mut blobs = Vec::with_capacity(sidecars.len());
    for (i, sidecar) in sidecars.iter().enumerate() {
        let hash = hashes.get(i).ok_or(anyhow::anyhow!("failed to get blob hash"))?;
        if sidecar.index as usize != hash.index {
            return Err(anyhow::anyhow!(
                "invalid sidecar ordering, blob hash index {} does not match sidecar index {}",
                hash.index,
                sidecar.index
            ));
        }

        // Ensure the blob's kzg commitment hashes to the expected value.
        if sidecar.to_kzg_versioned_hash() != hash.hash {
            return Err(anyhow::anyhow!(
                "expected hash {} for blob at index {} but got {:#?}",
                hash.hash,
                hash.index,
                sidecar.to_kzg_versioned_hash()
            ));
        }

        // Confirm blob data is valid by verifying its proof against the commitment
        match sidecar.verify_blob_kzg_proof() {
            Ok(true) => (),
            Ok(false) => {
                return Err(anyhow::anyhow!("blob at index {} failed verification", i));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("blob at index {} failed verification: {}", i, e));
            }
        }

        blobs.push(sidecar.blob);
    }
    Ok(blobs)
}

/// Minimal slot derivation implementation.
#[derive(Debug, Default)]
pub struct SimpleSlotDerivation;

impl SlotDerivation for SimpleSlotDerivation {
    fn slot(genesis: u64, slot_time: u64, timestamp: u64) -> anyhow::Result<u64> {
        if timestamp < genesis {
            return Err(anyhow::anyhow!(
                "provided timestamp ({}) precedes genesis time ({})",
                timestamp,
                genesis
            ));
        }
        Ok((timestamp - genesis) / slot_time)
    }
}

#[async_trait]
impl<T, B, S> BlobProvider for OnlineBlobProvider<T, B, S>
where
    T: Provider<Http<Client>> + Send,
    B: BeaconClient + Send + Sync,
    S: SlotDerivation + Send + Sync,
{
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: Vec<IndexedBlobHash>,
    ) -> anyhow::Result<Vec<Blob>> {
        let sidecars = self
            .get_blob_sidecars(block_ref, &blob_hashes)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        blobs_from_sidecars(&sidecars, &blob_hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::online::test_utils::{spawn_anvil, MockBeaconClient};

    #[tokio::test]
    async fn test_get_blob_sidecars_empty_hashes() {
        let (provider, _anvil) = spawn_anvil();
        let beacon_client = MockBeaconClient::default();
        let mut blob_provider: OnlineBlobProvider<_, _, SimpleSlotDerivation> =
            OnlineBlobProvider::new(provider, true, beacon_client, None, None);
        let block_ref = BlockInfo::default();
        let blob_hashes = Vec::new();
        let result = blob_provider.get_blob_sidecars(&block_ref, &blob_hashes).await;
        assert!(result.unwrap().is_empty());
    }
}
