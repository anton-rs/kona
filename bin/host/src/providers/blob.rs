//! Contains an online implementation of the `BlobProvider` trait.

use crate::providers::{BeaconClient, OnlineBeaconClient};
use alloy_eips::eip4844::{Blob, BlobTransactionSidecarItem};
use alloy_rpc_types_beacon::sidecar::BlobData;
use async_trait::async_trait;
use kona_derive::{errors::BlobProviderError, sources::IndexedBlobHash, traits::BlobProvider};
use op_alloy_protocol::BlockInfo;
use tracing::warn;

/// An online implementation of the [BlobProvider] trait.
#[derive(Debug, Clone)]
pub struct OnlineBlobProvider<B: BeaconClient> {
    /// The Beacon API client.
    beacon_client: B,
    /// Beacon Genesis time used for the time to slot conversion.
    pub genesis_time: Option<u64>,
    /// Slot interval used for the time to slot conversion.
    pub slot_interval: Option<u64>,
}

impl<B: BeaconClient> OnlineBlobProvider<B> {
    /// Creates a new instance of the [OnlineBlobProvider].
    ///
    /// The `genesis_time` and `slot_interval` arguments are _optional_ and the
    /// [OnlineBlobProvider] will attempt to load them dynamically at runtime if they are not
    /// provided.
    pub const fn new(
        beacon_client: B,
        genesis_time: Option<u64>,
        slot_interval: Option<u64>,
    ) -> Self {
        Self { beacon_client, genesis_time, slot_interval }
    }

    /// Loads the beacon genesis and config spec
    pub async fn load_configs(&mut self) -> Result<(), BlobProviderError> {
        if self.genesis_time.is_none() {
            self.genesis_time = Some(
                self.beacon_client
                    .beacon_genesis()
                    .await
                    .map_err(|e| BlobProviderError::Backend(e.to_string()))?
                    .data
                    .genesis_time,
            );
        }
        if self.slot_interval.is_none() {
            self.slot_interval = Some(
                self.beacon_client
                    .config_spec()
                    .await
                    .map_err(|e| BlobProviderError::Backend(e.to_string()))?
                    .data
                    .seconds_per_slot,
            );
        }
        Ok(())
    }

    /// Fetches blob sidecars for the given slot and blob hashes.
    pub async fn fetch_sidecars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobData>, BlobProviderError> {
        self.beacon_client
            .beacon_blob_side_cars(slot, hashes)
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

        // Extract the genesis timestamp and slot interval from the loaded configs.
        let genesis = self.genesis_time.expect("Genesis Config Loaded");
        let interval = self.slot_interval.expect("Config Spec Loaded");

        // Calculate the slot for the given timestamp.
        let slot = Self::slot(genesis, interval, block_ref.timestamp)?;

        // Fetch blob sidecars for the slot using the given blob hashes.
        let sidecars = self.fetch_sidecars(slot, blob_hashes).await?;

        // Filter blob sidecars that match the indicies in the specified list.
        let blob_hash_indicies = blob_hashes.iter().map(|b| b.index).collect::<Vec<usize>>();
        let filtered = sidecars
            .into_iter()
            .filter(|s| blob_hash_indicies.contains(&(s.index as usize)))
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
impl<B> BlobProvider for OnlineBlobProvider<B>
where
    B: BeaconClient + Send + Sync,
{
    type Error = BlobProviderError;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. The blobs are validated for their index and hashes using the specified
    /// [IndexedBlobHash].
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Box<Blob>>, Self::Error> {
        // Fetches the genesis timestamp and slot interval from the
        // [BeaconGenesis] and [ConfigSpec] if not previously loaded.
        self.load_configs().await?;

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
                    index: hash.index as u64,
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

/// The minimal interface required to fetch sidecars from a remote blob store.
#[async_trait]
pub trait BlobSidecarProvider {
    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    ///
    /// Consensus specs: <https://ethereum.github.io/beacon-APIs/#/Beacon/getBlobSidecars>
    async fn beacon_blob_side_cars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobData>, BlobProviderError>;
}

/// Blanket implementation of the [BlobSidecarProvider] trait for all types that
/// implemend [BeaconClient], which has a superset of the required functionality.
#[async_trait]
impl<B: BeaconClient + Send + Sync> BlobSidecarProvider for B {
    async fn beacon_blob_side_cars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobData>, BlobProviderError> {
        self.beacon_blob_side_cars(slot, hashes)
            .await
            .map_err(|e| BlobProviderError::Backend(e.to_string()))
    }
}

/// An online blob provider that optionally falls back to a secondary provider if the
/// primary fails to fetch blob sidecars.
///
/// This is useful for scenarios where blobs have been evicted from the primary provider's
/// blob store and need to be fetched from a remote archive API. The default eviction
/// policy on Ethereum is to keep blobs for 18 days.
///
/// Blob storage APIs are expected to implement the [BlobSidecarProvider] trait.
/// One example can be found at <https://github.com/base-org/blob-archiver>
#[derive(Debug, Clone)]
pub struct OnlineBlobProviderWithFallback<B: BeaconClient, F: BlobSidecarProvider> {
    primary: OnlineBlobProvider<B>,
    fallback: Option<F>,
}

impl<B: BeaconClient, F: BlobSidecarProvider> OnlineBlobProviderWithFallback<B, F> {
    /// Creates a new instance of the [OnlineBlobProviderWithFallback] with the
    /// specified primary and fallback providers.
    pub const fn new(primary: OnlineBlobProvider<B>, fallback: Option<F>) -> Self {
        Self { primary, fallback }
    }

    /// Attempts to fetch blob sidecars from the fallback provider, if configured.
    /// Calling this method without a fallback provider will return an error.
    async fn fallback_fetch_filtered_sidecars(
        &self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobTransactionSidecarItem>, BlobProviderError> {
        let Some(fallback) = self.fallback.as_ref() else {
            return Err(BlobProviderError::Backend(
                "cannot fetch blobs: the primary blob provider failed, and no fallback is configured".to_string()
            ));
        };

        if blob_hashes.is_empty() {
            return Ok(Vec::new());
        }

        // Extract the genesis timestamp and slot interval from the primary provider.
        let slot = OnlineBlobProvider::<B>::slot(
            self.primary.genesis_time.expect("Genesis Config Loaded"),
            self.primary.slot_interval.expect("Config Spec Loaded"),
            block_ref.timestamp,
        )?;

        // Fetch blob sidecars for the given block reference and blob hashes.
        let sidecars = fallback.beacon_blob_side_cars(slot, blob_hashes).await?;

        // Filter blob sidecars that match the indicies in the specified list.
        let blob_hash_indicies = blob_hashes.iter().map(|b| b.index).collect::<Vec<_>>();
        let filtered = sidecars
            .into_iter()
            .filter(|s| blob_hash_indicies.contains(&(s.index as usize)))
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
impl<B, F> BlobProvider for OnlineBlobProviderWithFallback<B, F>
where
    B: BeaconClient + Send + Sync,
    F: BlobSidecarProvider + Send + Sync,
{
    type Error = BlobProviderError;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. The blobs are validated for their index and hashes using the specified
    /// [IndexedBlobHash].
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Box<Blob>>, BlobProviderError> {
        match self.primary.get_blobs(block_ref, blob_hashes).await {
            Ok(blobs) => Ok(blobs),
            Err(primary_err) => {
                warn!(target: "blob_provider", "Primary provider failed: {:?}", primary_err);

                // Fetch the blob sidecars for the given block reference and blob hashes.
                let sidecars =
                    match self.fallback_fetch_filtered_sidecars(block_ref, blob_hashes).await {
                        Ok(sidecars) => sidecars,
                        Err(e) => {
                            warn!(target: "blob_provider", "Fallback provider failed: {:?}", e);
                            return Err(e);
                        }
                    };

                // Validate the blob sidecars straight away with the num hashes.
                let blobs = sidecars
                    .into_iter()
                    .enumerate()
                    .map(|(i, sidecar)| {
                        let hash = blob_hashes.get(i).ok_or(BlobProviderError::Backend(
                            "fallback: failed to get blob hash".to_string(),
                        ))?;
                        match sidecar.verify_blob(&alloy_eips::eip4844::IndexedBlobHash {
                            hash: hash.hash,
                            index: hash.index as u64,
                        }) {
                            Ok(_) => Ok(sidecar.blob),
                            Err(e) => Err(BlobProviderError::Backend(e.to_string())),
                        }
                    })
                    .collect::<Result<Vec<Box<Blob>>, BlobProviderError>>()?;
                Ok(blobs)
            }
        }
    }
}

/// A builder for a [OnlineBlobProviderWithFallback] instance.
///
/// This builder allows for the construction of a blob provider that
/// uses a primary beacon node and can fallback to a secondary [BlobSidecarProvider]
/// if the primary fails to fetch blob sidecars.
///
/// The fallback provider is optional and can be set using the [Self::with_fallback] method.
///
/// Two convenience methods are available for initializing the providers from beacon client URLs:
/// - [Self::with_primary] for the primary beacon client.
/// - [Self::with_fallback] for the fallback beacon client.
#[derive(Debug, Clone)]
pub struct OnlineBlobProviderBuilder<B: BeaconClient, F: BlobSidecarProvider> {
    beacon_client: Option<B>,
    fallback: Option<F>,
    genesis_time: Option<u64>,
    slot_interval: Option<u64>,
}

impl<B: BeaconClient, F: BlobSidecarProvider> Default for OnlineBlobProviderBuilder<B, F> {
    fn default() -> Self {
        Self { beacon_client: None, fallback: None, genesis_time: None, slot_interval: None }
    }
}

impl<B: BeaconClient, F: BlobSidecarProvider> OnlineBlobProviderBuilder<B, F> {
    /// Creates a new [OnlineBlobProviderBuilder].
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a primary beacon client to the builder. This is required.
    pub fn with_beacon_client(mut self, beacon_client: B) -> Self {
        self.beacon_client = Some(beacon_client);
        self
    }

    /// Adds a genesis time to the builder. This is optional.
    pub const fn with_genesis_time(mut self, genesis_time: u64) -> Self {
        self.genesis_time = Some(genesis_time);
        self
    }

    /// Adds a slot interval to the builder. This is optional.
    pub const fn with_slot_interval(mut self, slot_interval: u64) -> Self {
        self.slot_interval = Some(slot_interval);
        self
    }

    /// Adds a fallback blob provider to the builder. This is optional.
    pub fn with_fallback_provider(mut self, fallback: F) -> Self {
        self.fallback = Some(fallback);
        self
    }

    /// Builds the [OnlineBlobProviderWithFallback] instance.
    pub fn build(self) -> OnlineBlobProviderWithFallback<B, F> {
        self.into()
    }
}

impl<F: BlobSidecarProvider> OnlineBlobProviderBuilder<OnlineBeaconClient, F> {
    /// Adds a primary [OnlineBeaconClient] to the builder using the specified HTTP URL.
    pub fn with_primary(mut self, url: String) -> Self {
        self.beacon_client = Some(OnlineBeaconClient::new_http(url));
        self
    }
}

impl<B: BeaconClient + Send + Sync> OnlineBlobProviderBuilder<B, OnlineBeaconClient> {
    /// Adds a fallback [OnlineBeaconClient] to the builder using the specified HTTP URL.
    pub fn with_fallback(mut self, maybe_url: Option<String>) -> Self {
        self.fallback = maybe_url.map(OnlineBeaconClient::new_http);
        self
    }
}

impl<B: BeaconClient, F: BlobSidecarProvider> From<OnlineBlobProviderBuilder<B, F>>
    for OnlineBlobProviderWithFallback<B, F>
{
    fn from(builder: OnlineBlobProviderBuilder<B, F>) -> Self {
        Self::new(
            OnlineBlobProvider::new(
                builder.beacon_client.expect("Primary beacon client must be set"),
                builder.genesis_time,
                builder.slot_interval,
            ),
            builder.fallback,
        )
    }
}
