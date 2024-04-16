#![allow(dead_code)]
//! Contains an online implementation of the [BlobProvider] trait.

use crate::{
    traits::BlobProvider,
    types::{Blob, BlockInfo, IndexedBlobHash},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_provider::Provider;
use alloy_transport_http::Http;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use tracing::debug;

const (
	versionMethod        = "eth/v1/node/version"
	specMethod           = "eth/v1/config/spec"
	genesisMethod        = "eth/v1/beacon/genesis"
	sidecarsMethodPrefix = "eth/v1/beacon/blob_sidecars/"
)

/// The [BeaconClient] is a thin wrapper around the Beacon API.
pub trait BeaconClient {
    /// Returns the node version.
    fn node_version(&self) -> Result<String>;

    /// Returns the config spec.
    fn config_spec(&self) -> Result<APIConfigResponse>;

    /// Returns the beacon genesis.
    fn beacon_genesis(&self) -> Result<APIGenesisResponse>;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    fn beacon_blob_side_cars(
        &self,
        fetch_all_sidecars: bool,
        slot: u64,
        hashes: Vec<IndexedBlobHash>,
    ) -> Result<APIGetBlobSidecarsResponse>;
}

/// Specifies the derivation of a slot from a timestamp.
pub trait SlotDerivation {
    /// Converts a timestamp to a slot number.
    fn slot(genesis: u64, slot_time: u64, timestamp: u64) -> Result<u64>;
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
    pub fn load_configs(&mut self) -> Result<()> {
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
    pub async fn fetch_sidecars(&self, slot: u64, hashes: Vec<IndexedBlobHash>) -> Result<BlobSidecar> {
        unimplemented!("fetching blob sidecars is not implemented");
    }


    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    pub async fn get_blob_sidecars(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: Vec<IndexedBlobHash>,
    ) -> Result<Vec<Blob>> {
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

        let blob_hash_indicies = blob_hashes.iter().map(|b| b.index).collect::<Vec<_>>();
        let filtered = sidecars.iter().filter(|s| blob_hashes.contains(s.index)).collect::<Vec<_>>();

        if filtered.len() != blob_hashes.len() {
            
        }

        // TODO: implement
        Ok(Vec::new())
    }
}

#[async_trait]
impl<T: Provider<Http<Client>>> BlobProvider for OnlineBlobProvider<T> {
    async fn get_blobs(
        &self,
        _block_ref: &BlockInfo,
        _blob_hashes: Vec<IndexedBlobHash>,
    ) -> Result<Vec<Blob>> {
        unimplemented!("TODO: Implement OnlineBlobProvider::get_blobs")
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

// GetTimeToSlotFn returns a function that converts a timestamp to a slot number.
func (cl *L1BeaconClient) GetTimeToSlotFn(ctx context.Context) (TimeToSlotFn, error) {
	cl.initLock.Lock()
	defer cl.initLock.Unlock()
	if cl.timeToSlotFn != nil {
		return cl.timeToSlotFn, nil
	}

	genesis, err := cl.cl.BeaconGenesis(ctx)
	if err != nil {
		return nil, err
	}

	config, err := cl.cl.ConfigSpec(ctx)
	if err != nil {
		return nil, err
	}

	genesisTime := uint64(genesis.Data.GenesisTime)
	secondsPerSlot := uint64(config.Data.SecondsPerSlot)
	if secondsPerSlot == 0 {
		return nil, fmt.Errorf("got bad value for seconds per slot: %v", config.Data.SecondsPerSlot)
	}
	cl.timeToSlotFn = func(timestamp uint64) (uint64, error) {
		if timestamp < genesisTime {
			return 0, fmt.Errorf("provided timestamp (%v) precedes genesis time (%v)", timestamp, genesisTime)
		}
		return (timestamp - genesisTime) / secondsPerSlot, nil
	}
	return cl.timeToSlotFn, nil
}


// GetBlobSidecars fetches blob sidecars that were confirmed in the specified
// L1 block with the given indexed hashes.
// Order of the returned sidecars is guaranteed to be that of the hashes.
// Blob data is not checked for validity.

// func (cl *L1BeaconClient) GetBlobSidecars(ctx context.Context, ref eth.L1BlockRef, hashes
// []eth.IndexedBlobHash) ([]*eth.BlobSidecar, error) { 	if len(hashes) == 0 {
// 		return []*eth.BlobSidecar{}, nil
// 	}

// 	slotFn, err := cl.GetTimeToSlotFn(ctx)
// 	if err != nil {
// 		return nil, fmt.Errorf("failed to get time to slot function: %w", err)
// 	}
// 	slot, err := slotFn(ref.Time)
// 	if err != nil {
// 		return nil, fmt.Errorf("error in converting ref.Time to slot: %w", err)
// 	}
//
// 	resp, err := cl.fetchSidecars(ctx, slot, hashes)
// 	if err != nil {
// 		return nil, fmt.Errorf("failed to fetch blob sidecars for slot %v block %v: %w", slot, ref, err)
// 	}
//
// 	apiscs := make([]*eth.APIBlobSidecar, 0, len(hashes))
// 	// filter and order by hashes
// 	for _, h := range hashes {
// 		for _, apisc := range resp.Data {
// 			if h.Index == uint64(apisc.Index) {
// 				apiscs = append(apiscs, apisc)
// 				break
// 			}
// 		}
// 	}
//
// 	if len(hashes) != len(apiscs) {
// 		return nil, fmt.Errorf("expected %v sidecars but got %v", len(hashes), len(apiscs))
// 	}
//
// 	bscs := make([]*eth.BlobSidecar, 0, len(hashes))
// 	for _, apisc := range apiscs {
// 		bscs = append(bscs, apisc.BlobSidecar())
// 	}
//
// 	return bscs, nil
// }
