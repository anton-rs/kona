//! Contains an online implementation of the [BlobProvider] trait.

use crate::{
    online::BeaconClient,
    traits::BlobProvider,
    types::{APIBlobSidecar, Blob, BlobProviderError, BlobSidecar, BlockInfo, IndexedBlobHash},
};
use alloc::{boxed::Box, vec::Vec};
use async_trait::async_trait;
use core::marker::PhantomData;
use tracing::debug;

/// Specifies the derivation of a slot from a timestamp.
pub trait SlotDerivation {
    /// Converts a timestamp to a slot number.
    fn slot(genesis: u64, slot_time: u64, timestamp: u64) -> anyhow::Result<u64>;
}

/// An online implementation of the [BlobProvider] trait.
#[derive(Debug, Clone)]
pub struct OnlineBlobProvider<B: BeaconClient, S: SlotDerivation> {
    /// Whether to fetch all sidecars.
    fetch_all_sidecars: bool,
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
    pub fn new(
        fetch_all_sidecars: bool,
        beacon_client: B,
        genesis_time: Option<u64>,
        slot_interval: Option<u64>,
    ) -> Self {
        Self {
            fetch_all_sidecars,
            beacon_client,
            genesis_time,
            slot_interval,
            _slot_derivation: PhantomData,
        }
    }

    /// Loads the beacon genesis and config spec
    pub async fn load_configs(&mut self) -> Result<(), BlobProviderError> {
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
    ) -> Result<Vec<APIBlobSidecar>, BlobProviderError> {
        self.beacon_client
            .beacon_blob_side_cars(self.fetch_all_sidecars, slot, hashes)
            .await
            .map(|r| r.data)
            .map_err(|e| e.into())
    }
}

/// Minimal slot derivation implementation.
#[derive(Debug, Default, Clone)]
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
impl<B, S> BlobProvider for OnlineBlobProvider<B, S>
where
    B: BeaconClient + Send + Sync,
    S: SlotDerivation + Send + Sync,
{
    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. The blobs are validated for their index and hashes using the specified
    /// [IndexedBlobHash].
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Blob>, BlobProviderError> {
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
        let slot =
            S::slot(genesis, interval, block_ref.timestamp).map_err(BlobProviderError::Slot)?;

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

        // Validate the blob sidecars straight away with the `IndexedBlobHash`es.
        let sidecars = filtered.into_iter().map(|s| s.inner).collect::<Vec<BlobSidecar>>();
        let blobs = sidecars
            .into_iter()
            .enumerate()
            .map(|(i, sidecar)| {
                let hash =
                    blob_hashes.get(i).ok_or_else(|| anyhow::anyhow!("failed to get blob hash"))?;
                match sidecar.verify_blob(hash) {
                    Ok(_) => Ok(sidecar.blob),
                    Err(e) => Err(e),
                }
            })
            .collect::<anyhow::Result<Vec<Blob>>>()?;

        Ok(blobs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        online::test_utils::MockBeaconClient,
        types::{APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse},
    };
    use alloc::vec;
    use alloy_primitives::b256;

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
            OnlineBlobProvider::new(true, beacon_client, None, None);
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
            OnlineBlobProvider::new(true, beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blobs = blob_provider.get_blobs(&block_ref, &blob_hashes).await.unwrap();
        assert_eq!(blobs.len(), 5);
    }

    #[tokio::test]
    async fn test_get_blobs_empty_hashes() {
        let beacon_client = MockBeaconClient::default();
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(true, beacon_client, None, None);
        let block_ref = BlockInfo::default();
        let blob_hashes = Vec::new();
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_blobs_beacon_genesis_fetch_fails() {
        let beacon_client = MockBeaconClient::default();
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(true, beacon_client, None, None);
        let block_ref = BlockInfo::default();
        let blob_hashes = vec![IndexedBlobHash::default()];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Custom(anyhow::anyhow!("failed to get beacon genesis"))
        );
    }

    #[tokio::test]
    async fn test_get_blobs_config_spec_fetch_fails() {
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::default()),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(true, beacon_client, None, None);
        let block_ref = BlockInfo::default();
        let blob_hashes = vec![IndexedBlobHash::default()];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Custom(anyhow::anyhow!("failed to get config spec"))
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
            OnlineBlobProvider::new(true, beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 5, ..Default::default() };
        let blob_hashes = vec![IndexedBlobHash::default()];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Slot(anyhow::anyhow!(
                "provided timestamp (5) precedes genesis time (10)"
            ))
        );
    }

    #[tokio::test]
    async fn test_get_blob_sidecars_fetch_fails() {
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(true, beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blob_hashes = vec![IndexedBlobHash::default()];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Custom(anyhow::anyhow!("blob_sidecars not set"))
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
            OnlineBlobProvider::new(true, beacon_client, None, None);
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
            OnlineBlobProvider::new(true, beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Custom(anyhow::anyhow!(
                "invalid sidecar ordering, blob hash index 4 does not match sidecar index 0"
            ))
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
            OnlineBlobProvider::new(true, beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blob_hashes = vec![IndexedBlobHash {
            hash: alloy_primitives::FixedBytes::from([1; 32]),
            ..Default::default()
        }];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(result.unwrap_err(), BlobProviderError::Custom(anyhow::anyhow!("expected hash 0x0101010101010101010101010101010101010101010101010101010101010101 for blob at index 0 but got 0x01b0761f87b081d5cf10757ccc89f12be355c70e2e29df288b65b30710dcbcd1")));
    }

    #[tokio::test]
    async fn test_get_blobs_failed_verification() {
        let beacon_client = MockBeaconClient {
            beacon_genesis: Some(APIGenesisResponse::new(10)),
            config_spec: Some(APIConfigResponse::new(12)),
            blob_sidecars: Some(APIGetBlobSidecarsResponse {
                data: vec![APIBlobSidecar { inner: BlobSidecar::default(), ..Default::default() }],
            }),
            ..Default::default()
        };
        let mut blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
            OnlineBlobProvider::new(true, beacon_client, None, None);
        let block_ref = BlockInfo { timestamp: 15, ..Default::default() };
        let blob_hashes = vec![IndexedBlobHash {
            hash: b256!("01b0761f87b081d5cf10757ccc89f12be355c70e2e29df288b65b30710dcbcd1"),
            ..Default::default()
        }];
        let result = blob_provider.get_blobs(&block_ref, &blob_hashes).await;
        assert_eq!(
            result.unwrap_err(),
            BlobProviderError::Custom(anyhow::anyhow!("blob at index 0 failed verification"))
        );
    }
}
