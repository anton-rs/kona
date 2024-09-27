//! Contains an online implementation of the [BlobProvider] trait.

use crate::{
    ensure,
    errors::BlobProviderError,
    online::{BeaconClient, OnlineBeaconClient},
    traits::BlobProvider,
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use alloy_eips::eip4844::Blob;
use async_trait::async_trait;
use core::marker::PhantomData;
use kona_primitives::{APIBlobSidecar, BlobSidecar, IndexedBlobHash};
use op_alloy_protocol::BlockInfo;
use tracing::warn;

/// Specifies the derivation of a slot from a timestamp.
pub trait SlotDerivation {
    /// Converts a timestamp to a slot number.
    fn slot(genesis: u64, slot_time: u64, timestamp: u64) -> Result<u64, BlobProviderError>;
}

/// An online implementation of the [BlobProvider] trait.
#[derive(Debug, Clone)]
pub struct OnlineBlobProvider<B: BeaconClient, S: SlotDerivation> {
    /// The Beacon API client.
    beacon_client: B,
    /// Beacon Genesis time used for the time to slot conversion.
    pub genesis_time: Option<u64>,
    /// Slot interval used for the time to slot conversion.
    pub slot_interval: Option<u64>,
    /// Phantom data for slot derivation.
    _slot_derivation: PhantomData<S>,
}

impl<B: BeaconClient, S: SlotDerivation> OnlineBlobProvider<B, S> {
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
        Self { beacon_client, genesis_time, slot_interval, _slot_derivation: PhantomData }
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
    ) -> Result<Vec<APIBlobSidecar>, BlobProviderError> {
        self.beacon_client
            .beacon_blob_side_cars(slot, hashes)
            .await
            .map_err(|e| BlobProviderError::Backend(e.to_string()))
    }

    /// Fetches blob sidecars for the given block reference and blob hashes.
    pub async fn fetch_filtered_sidecars(
        &self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobSidecar>, BlobProviderError> {
        if blob_hashes.is_empty() {
            return Ok(Vec::new());
        }

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
            return Err(BlobProviderError::SidecarLengthMismatch(blob_hashes.len(), filtered.len()));
        }

        Ok(filtered.into_iter().map(|s| s.inner).collect::<Vec<BlobSidecar>>())
    }
}

/// Minimal slot derivation implementation.
#[derive(Debug, Default, Clone)]
pub struct SimpleSlotDerivation;

impl SlotDerivation for SimpleSlotDerivation {
    fn slot(genesis: u64, slot_time: u64, timestamp: u64) -> Result<u64, BlobProviderError> {
        ensure!(timestamp >= genesis, BlobProviderError::SlotDerivation);
        Ok((timestamp - genesis) / slot_time)
    }
}

#[async_trait]
impl<B, S> BlobProvider for OnlineBlobProvider<B, S>
where
    B: BeaconClient + Send + Sync,
    S: SlotDerivation + Send + Sync,
{
    type Error = BlobProviderError;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. The blobs are validated for their index and hashes using the specified
    /// [IndexedBlobHash].
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Blob>, Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["blob_provider", "get_blobs"]);
        crate::timer!(START, PROVIDER_RESPONSE_TIME, &["blob_provider", "get_blobs"], timer);
        // Fetches the genesis timestamp and slot interval from the
        // [BeaconGenesis] and [ConfigSpec] if not previously loaded.
        if let Err(e) = self.load_configs().await {
            crate::timer!(DISCARD, timer);
            crate::inc!(PROVIDER_ERRORS, &["blob_provider", "get_blobs", "load_configs"]);
            return Err(e);
        }

        // Fetch the blob sidecars for the given block reference and blob hashes.
        let sidecars = match self.fetch_filtered_sidecars(block_ref, blob_hashes).await {
            Ok(sidecars) => sidecars,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["blob_provider", "get_blobs", "fetch_filtered_sidecars"]
                );
                return Err(e);
            }
        };

        // Validate the blob sidecars straight away with the `IndexedBlobHash`es.
        let blobs = match sidecars
            .into_iter()
            .enumerate()
            .map(|(i, sidecar)| {
                let hash = blob_hashes
                    .get(i)
                    .ok_or(BlobProviderError::Backend("Missing blob hash".to_string()))?;
                match sidecar.verify_blob(hash) {
                    Ok(_) => Ok(sidecar.blob),
                    Err(e) => Err(BlobProviderError::Backend(e.to_string())),
                }
            })
            .collect::<Result<Vec<Blob>, BlobProviderError>>()
        {
            Ok(blobs) => blobs,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["blob_provider", "get_blobs", "verify_blob"]);
                return Err(BlobProviderError::Backend(e.to_string()));
            }
        };

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
    ) -> Result<Vec<APIBlobSidecar>, BlobProviderError>;
}

/// Blanket implementation of the [BlobSidecarProvider] trait for all types that
/// implemend [BeaconClient], which has a superset of the required functionality.
#[async_trait]
impl<B: BeaconClient + Send + Sync> BlobSidecarProvider for B {
    async fn beacon_blob_side_cars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<APIBlobSidecar>, BlobProviderError> {
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
pub struct OnlineBlobProviderWithFallback<
    B: BeaconClient,
    F: BlobSidecarProvider,
    S: SlotDerivation,
> {
    primary: OnlineBlobProvider<B, S>,
    fallback: Option<F>,
    _slot_derivation: PhantomData<S>,
}

impl<B: BeaconClient, F: BlobSidecarProvider, S: SlotDerivation>
    OnlineBlobProviderWithFallback<B, F, S>
{
    /// Creates a new instance of the [OnlineBlobProviderWithFallback] with the
    /// specified primary and fallback providers.
    pub const fn new(primary: OnlineBlobProvider<B, S>, fallback: Option<F>) -> Self {
        Self { primary, fallback, _slot_derivation: PhantomData }
    }

    /// Attempts to fetch blob sidecars from the fallback provider, if configured.
    /// Calling this method without a fallback provider will return an error.
    async fn fallback_fetch_filtered_sidecars(
        &self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<BlobSidecar>, BlobProviderError> {
        let Some(fallback) = self.fallback.as_ref() else {
            return Err(BlobProviderError::Backend(
                "cannot fetch blobs: the primary blob provider failed, and no fallback is configured".to_string()
            ));
        };

        if blob_hashes.is_empty() {
            return Ok(Vec::new());
        }

        // Extract the genesis timestamp and slot interval from the primary provider.
        let slot = S::slot(
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
            .filter(|s| blob_hash_indicies.contains(&(s.inner.index as usize)))
            .collect::<Vec<_>>();

        // Validate the correct number of blob sidecars were retrieved.
        if blob_hashes.len() != filtered.len() {
            return Err(BlobProviderError::SidecarLengthMismatch(blob_hashes.len(), filtered.len()));
        }

        Ok(filtered.into_iter().map(|s| s.inner).collect::<Vec<BlobSidecar>>())
    }
}

#[async_trait]
impl<B, F, S> BlobProvider for OnlineBlobProviderWithFallback<B, F, S>
where
    B: BeaconClient + Send + Sync,
    F: BlobSidecarProvider + Send + Sync,
    S: SlotDerivation + Send + Sync,
{
    type Error = BlobProviderError;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. The blobs are validated for their index and hashes using the specified
    /// [IndexedBlobHash].
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Blob>, BlobProviderError> {
        match self.primary.get_blobs(block_ref, blob_hashes).await {
            Ok(blobs) => Ok(blobs),
            Err(primary_err) => {
                crate::inc!(PROVIDER_ERRORS, &["blob_provider", "get_blobs", "primary"]);
                warn!(target: "blob_provider", "Primary provider failed: {:?}", primary_err);

                // Fetch the blob sidecars for the given block reference and blob hashes.
                let sidecars =
                    match self.fallback_fetch_filtered_sidecars(block_ref, blob_hashes).await {
                        Ok(sidecars) => sidecars,
                        Err(e) => {
                            warn!(target: "blob_provider", "Fallback provider failed: {:?}", e);
                            crate::inc!(
                                PROVIDER_ERRORS,
                                &["blob_provider", "get_blobs", "fallback_fetch_filtered_sidecars"]
                            );
                            return Err(e);
                        }
                    };

                // Validate the blob sidecars straight away with the `IndexedBlobHash`es.
                let blobs = match sidecars
                    .into_iter()
                    .enumerate()
                    .map(|(i, sidecar)| {
                        let hash = blob_hashes.get(i).ok_or(BlobProviderError::Backend(
                            "fallback: failed to get blob hash".to_string(),
                        ))?;
                        match sidecar.verify_blob(hash) {
                            Ok(_) => Ok(sidecar.blob),
                            Err(e) => Err(BlobProviderError::Backend(e.to_string())),
                        }
                    })
                    .collect::<Result<Vec<Blob>, BlobProviderError>>()
                {
                    Ok(blobs) => blobs,
                    Err(e) => {
                        crate::inc!(
                            PROVIDER_ERRORS,
                            &["blob_provider", "get_blobs", "fallback_verify_blob"]
                        );
                        return Err(e);
                    }
                };

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
pub struct OnlineBlobProviderBuilder<B: BeaconClient, F: BlobSidecarProvider, S: SlotDerivation> {
    beacon_client: Option<B>,
    fallback: Option<F>,
    genesis_time: Option<u64>,
    slot_interval: Option<u64>,
    _slot_derivation: PhantomData<S>,
}

impl<B: BeaconClient, F: BlobSidecarProvider, S: SlotDerivation> Default
    for OnlineBlobProviderBuilder<B, F, S>
{
    fn default() -> Self {
        Self {
            beacon_client: None,
            fallback: None,
            genesis_time: None,
            slot_interval: None,
            _slot_derivation: PhantomData,
        }
    }
}

impl<B: BeaconClient, F: BlobSidecarProvider, S: SlotDerivation>
    OnlineBlobProviderBuilder<B, F, S>
{
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
    pub fn build(self) -> OnlineBlobProviderWithFallback<B, F, S> {
        self.into()
    }
}

impl<F: BlobSidecarProvider, S: SlotDerivation + Sync>
    OnlineBlobProviderBuilder<OnlineBeaconClient, F, S>
{
    /// Adds a primary [OnlineBeaconClient] to the builder using the specified HTTP URL.
    pub fn with_primary(mut self, url: String) -> Self {
        self.beacon_client = Some(OnlineBeaconClient::new_http(url));
        self
    }
}

impl<B: BeaconClient + Send + Sync, S: SlotDerivation + Sync>
    OnlineBlobProviderBuilder<B, OnlineBeaconClient, S>
{
    /// Adds a fallback [OnlineBeaconClient] to the builder using the specified HTTP URL.
    pub fn with_fallback(mut self, maybe_url: Option<String>) -> Self {
        self.fallback = maybe_url.map(OnlineBeaconClient::new_http);
        self
    }
}

impl<B: BeaconClient, F: BlobSidecarProvider, S: SlotDerivation>
    From<OnlineBlobProviderBuilder<B, F, S>> for OnlineBlobProviderWithFallback<B, F, S>
{
    fn from(builder: OnlineBlobProviderBuilder<B, F, S>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::online::test_utils::MockBeaconClient;
    use alloc::vec;
    use alloy_primitives::b256;
    use kona_primitives::{APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse};

    #[tokio::test]
    async fn test_load_config_succeeds() {
        let genesis_time = 10;
        let seconds_per_slot = 12;
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(genesis_time)),
            config_spec: Some(APIConfigResponse::new(seconds_per_slot)),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let result = blob_provider.load_configs().await;
        assert!(result.is_ok());
        assert_eq!(blob_provider.genesis_time, Some(genesis_time));
        assert_eq!(blob_provider.slot_interval, Some(seconds_per_slot));
    }

    #[tokio::test]
    async fn test_get_blobs() {
        let json_bytes = include_bytes!("testdata/eth_v1_beacon_sidecars_goerli.json");
        let sidecars: APIGetBlobSidecarsResponse = serde_json::from_slice(json_bytes).unwrap();
        let blob_hashes = vec![
            IndexedBlobHash {
                index: 0,
                hash: b256!("011075cbb20f3235b3179a5dff22689c410cd091692180f4b6a12be77ea0f586"),
            },
            IndexedBlobHash {
                index: 1,
                hash: b256!("010a9e10aab79bab62e10a5b83c164a91451b6ef56d31ac95a9514ffe6d6b4e6"),
            },
            IndexedBlobHash {
                index: 2,
                hash: b256!("016122c8e41c69917b688240707d107aa6d2a480343e4e323e564241769a6b4a"),
            },
            IndexedBlobHash {
                index: 3,
                hash: b256!("01df1f9ae707f5847513c9c430b683182079edf2b1f94ee12e4daae7f3c8c309"),
            },
            IndexedBlobHash {
                index: 4,
                hash: b256!("01e5ee2f6cbbafb3c03f05f340e795fe5b5a8edbcc9ac3fc7bd3d1940b99ef3c"),
            },
        ];
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            blob_sidecars: Some(sidecars),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blobs = blob_provider.get_blobs(&block_ref, &blob_hashes).await.unwrap();
        assert_eq!(blobs.len(), 5);
    }

    #[tokio::test]
    async fn test_get_blobs_empty_hashes() {
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let block_ref = BlockInfo::default();
        let blob_hashes = Vec::new();
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_blobs_beacon_genesis_fetch_fails() {
        let beacon_client = MockBeaconClient::default();
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let block_ref = BlockInfo::default();
        let blob_hashes = vec![IndexedBlobHash::default()];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Backend("beacon_genesis not set".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_blobs_config_spec_fetch_fails() {
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::default()),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let block_ref = BlockInfo::default();
        let blob_hashes = vec![IndexedBlobHash::default()];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Backend("config_spec not set".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_blobs_before_genesis_fails() {
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 5, ..Default::default() };
        let blob_hashes = vec![IndexedBlobHash::default()];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(result.unwrap_err(), BlobProviderError::SlotDerivation);
    }

    #[tokio::test]
    async fn test_get_blob_sidecars_fetch_fails() {
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blob_hashes = vec![IndexedBlobHash::default()];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Backend("blob_sidecars not set".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_blob_sidecars_length_mismatch() {
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            blob_sidecars: Some(APIGetBlobSidecarsResponse {
                data: vec![APIBlobSidecar::default()],
            }),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blob_hashes = vec![IndexedBlobHash { index: 1, ..Default::default() }];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(result.unwrap_err(), BlobProviderError::SidecarLengthMismatch(1, 0));
    }

    #[tokio::test]
    async fn test_get_blobs_invalid_ordering() {
        let json_bytes = include_bytes!("testdata/eth_v1_beacon_sidecars_goerli.json");
        let sidecars: APIGetBlobSidecarsResponse = serde_json::from_slice(json_bytes).unwrap();
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            blob_sidecars: Some(sidecars),
            ..Default::default()
        };
        let blob_hashes = vec![
            IndexedBlobHash {
                index: 4,
                hash: b256!("01e5ee2f6cbbafb3c03f05f340e795fe5b5a8edbcc9ac3fc7bd3d1940b99ef3c"),
            },
            IndexedBlobHash {
                index: 0,
                hash: b256!("011075cbb20f3235b3179a5dff22689c410cd091692180f4b6a12be77ea0f586"),
            },
            IndexedBlobHash {
                index: 1,
                hash: b256!("010a9e10aab79bab62e10a5b83c164a91451b6ef56d31ac95a9514ffe6d6b4e6"),
            },
            IndexedBlobHash {
                index: 2,
                hash: b256!("016122c8e41c69917b688240707d107aa6d2a480343e4e323e564241769a6b4a"),
            },
            IndexedBlobHash {
                index: 3,
                hash: b256!("01df1f9ae707f5847513c9c430b683182079edf2b1f94ee12e4daae7f3c8c309"),
            },
        ];
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Backend(
                "invalid sidecar ordering, blob hash index 4 does not match sidecar index 0"
                    .to_string()
            )
        );
    }

    #[tokio::test]
    async fn test_get_blobs_invalid_hash() {
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            blob_sidecars: Some(APIGetBlobSidecarsResponse {
                data: vec![APIBlobSidecar { inner: BlobSidecar::default(), ..Default::default() }],
            }),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blob_hashes = vec![IndexedBlobHash {
            hash: alloy_primitives::FixedBytes::from([1; 32]),
            ..Default::default()
        }];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(result.unwrap_err(), BlobProviderError::Backend("expected hash 0x0101010101010101010101010101010101010101010101010101010101010101 for blob at index 0 but got 0x01b0761f87b081d5cf10757ccc89f12be355c70e2e29df288b65b30710dcbcd1".to_string()));
    }

    #[test]
    fn test_get_blobs_failed_verification() {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_stack_size(8 * 32 * 32 * 1024) // 8 MiB
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let beacon_client = MockBeaconClient {
                    beacon_genesis: Some(APIGenesisResponse::new(10)),
                    config_spec: Some(APIConfigResponse::new(12)),
                    blob_sidecars: Some(APIGetBlobSidecarsResponse {
                        data: vec![APIBlobSidecar {
                            inner: BlobSidecar::default(),
                            ..Default::default()
                        }],
                    }),
                    ..Default::default()
                };
                let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
                    OnlineBlobProvider::new(beacon_client, None, None);
                let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
                let blob_hashes = vec![IndexedBlobHash {
                    hash: b256!("01b0761f87b081d5cf10757ccc89f12be355c70e2e29df288b65b30710dcbcd1"),
                    ..Default::default()
                }];
                let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
                assert_eq!(
                    result,
                    Err(BlobProviderError::Backend(
                        "blob at index 0 failed verification".to_string()
                    ))
                );
            })
    }

    #[tokio::test]
    async fn test_get_blob_fallback() {
        let json_bytes = include_bytes!("testdata/eth_v1_beacon_sidecars_goerli.json");
        let sidecars: APIGetBlobSidecarsResponse = serde_json::from_slice(json_bytes).unwrap();

        // Provide no sidecars to the primary provider to trigger a fallback fetch
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            blob_sidecars: None,
            ..Default::default()
        };
        let fallback_client =
            MockBeaconClient { blob_sidecars: Some(sidecars), ..Default::default() };
        let mut blob_provider: OnlineBlobProviderWithFallback<_, _, SimpleSlotDerivation> =
            OnlineBlobProviderWithFallback::new(
                OnlineBlobProvider::new(beacon_client, None, None),
                Some(fallback_client),
            );
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blob_hashes = vec![
            IndexedBlobHash {
                index: 0,
                hash: b256!("011075cbb20f3235b3179a5dff22689c410cd091692180f4b6a12be77ea0f586"),
            },
            IndexedBlobHash {
                index: 1,
                hash: b256!("010a9e10aab79bab62e10a5b83c164a91451b6ef56d31ac95a9514ffe6d6b4e6"),
            },
            IndexedBlobHash {
                index: 2,
                hash: b256!("016122c8e41c69917b688240707d107aa6d2a480343e4e323e564241769a6b4a"),
            },
            IndexedBlobHash {
                index: 3,
                hash: b256!("01df1f9ae707f5847513c9c430b683182079edf2b1f94ee12e4daae7f3c8c309"),
            },
            IndexedBlobHash {
                index: 4,
                hash: b256!("01e5ee2f6cbbafb3c03f05f340e795fe5b5a8edbcc9ac3fc7bd3d1940b99ef3c"),
            },
        ];
        let blobs = blob_provider.get_blobs(&block_ref, &blob_hashes).await.unwrap();
        assert_eq!(blobs.len(), 5);
    }

    #[tokio::test]
    async fn test_get_blobs_fallback_partial_sidecar() {
        let json_bytes = include_bytes!("testdata/eth_v1_beacon_sidecars_goerli.json");
        let all_sidecars: APIGetBlobSidecarsResponse = serde_json::from_slice(json_bytes).unwrap();

        let online_sidecars = APIGetBlobSidecarsResponse {
            // Remove some sidecars from the online provider to trigger a fallback fetch
            data: all_sidecars.data.clone().into_iter().take(2).collect::<Vec<_>>(),
        };

        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            blob_sidecars: Some(online_sidecars),
            ..Default::default()
        };
        let fallback_client =
            MockBeaconClient { blob_sidecars: Some(all_sidecars), ..Default::default() };
        let mut blob_provider: OnlineBlobProviderWithFallback<_, _, SimpleSlotDerivation> =
            OnlineBlobProviderWithFallback::new(
                OnlineBlobProvider::new(beacon_client, None, None),
                Some(fallback_client),
            );
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blob_hashes = vec![
            IndexedBlobHash {
                index: 0,
                hash: b256!("011075cbb20f3235b3179a5dff22689c410cd091692180f4b6a12be77ea0f586"),
            },
            IndexedBlobHash {
                index: 1,
                hash: b256!("010a9e10aab79bab62e10a5b83c164a91451b6ef56d31ac95a9514ffe6d6b4e6"),
            },
            IndexedBlobHash {
                index: 2,
                hash: b256!("016122c8e41c69917b688240707d107aa6d2a480343e4e323e564241769a6b4a"),
            },
            IndexedBlobHash {
                index: 3,
                hash: b256!("01df1f9ae707f5847513c9c430b683182079edf2b1f94ee12e4daae7f3c8c309"),
            },
            IndexedBlobHash {
                index: 4,
                hash: b256!("01e5ee2f6cbbafb3c03f05f340e795fe5b5a8edbcc9ac3fc7bd3d1940b99ef3c"),
            },
        ];
        let blobs = blob_provider.get_blobs(&block_ref, &blob_hashes).await.unwrap();
        assert_eq!(blobs.len(), 5);
    }
}
