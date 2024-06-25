//! Contains an online implementation of the [BeaconClient] trait.

use crate::types::{
    APIBlobSidecar, APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse,
    IndexedBlobHash,
};
use alloc::{boxed::Box, format, string::String, vec::Vec};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;

/// The config spec engine api method.
pub(crate) const SPEC_METHOD: &str = "eth/v1/config/spec";

/// The beacon genesis engine api method.
pub(crate) const GENESIS_METHOD: &str = "eth/v1/beacon/genesis";

/// The blob sidecars engine api method prefix.
pub(crate) const SIDECARS_METHOD_PREFIX: &str = "eth/v1/beacon/blob_sidecars";

/// The [BeaconClient] is a thin wrapper around the Beacon API.
#[async_trait]
pub trait BeaconClient {
    /// Returns the config spec.
    async fn config_spec(&self) -> Result<APIConfigResponse>;

    /// Returns the beacon genesis.
    async fn beacon_genesis(&self) -> Result<APIGenesisResponse>;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    async fn beacon_blob_side_cars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<APIBlobSidecar>>;
}

/// An online implementation of the [BeaconClient] trait.
#[derive(Debug, Clone)]
pub struct OnlineBeaconClient {
    /// The base URL of the beacon API.
    base: String,
    /// The inner reqwest client.
    inner: Client,
}

impl OnlineBeaconClient {
    /// Creates a new [OnlineBeaconClient] from the provided [reqwest::Url].
    pub fn new_http(base: String) -> Self {
        Self { base, inner: Client::new() }
    }
}

#[async_trait]
impl BeaconClient for OnlineBeaconClient {
    async fn config_spec(&self) -> Result<APIConfigResponse> {
        let timer = crate::online::metrics::PROVIDER_RESPONSE_TIME
            .with_label_values(&["beacon_client", "config_spec"])
            .start_timer();
        let first = match self
            .inner
            .get(format!("{}/{}", self.base, SPEC_METHOD))
            .send()
            .await
            .map_err(|e| anyhow!(e))
        {
            Ok(response) => response,
            Err(e) => {
                timer.observe_duration();
                return Err(e);
            }
        };
        let res = first.json::<APIConfigResponse>().await.map_err(|e| anyhow!(e));
        timer.observe_duration();
        res
    }

    async fn beacon_genesis(&self) -> Result<APIGenesisResponse> {
        let timer = crate::online::metrics::PROVIDER_RESPONSE_TIME
            .with_label_values(&["beacon_client", "beacon_genesis"])
            .start_timer();
        let first = match self
            .inner
            .get(format!("{}/{}", self.base, GENESIS_METHOD))
            .send()
            .await
            .map_err(|e| anyhow!(e))
        {
            Ok(response) => response,
            Err(e) => {
                timer.observe_duration();
                return Err(e);
            }
        };
        let res = first.json::<APIGenesisResponse>().await.map_err(|e| anyhow!(e));
        timer.observe_duration();
        res
    }

    async fn beacon_blob_side_cars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<APIBlobSidecar>> {
        let timer = crate::online::metrics::PROVIDER_RESPONSE_TIME
            .with_label_values(&["beacon_client", "beacon_blob_side_cars"])
            .start_timer();
        let raw_response = match self
            .inner
            .get(format!("{}/{}/{}", self.base, SIDECARS_METHOD_PREFIX, slot))
            .send()
            .await
            .map_err(|e| anyhow!(e))
        {
            Ok(response) => response,
            Err(e) => {
                timer.observe_duration();
                return Err(e);
            }
        };
        let raw_response =
            match raw_response.json::<APIGetBlobSidecarsResponse>().await.map_err(|e| anyhow!(e)) {
                Ok(response) => response,
                Err(e) => {
                    timer.observe_duration();
                    return Err(e);
                }
            };

        let mut sidecars = Vec::with_capacity(hashes.len());

        // Filter the sidecars by the hashes, in-order.
        hashes.iter().for_each(|hash| {
            if let Some(sidecar) =
                raw_response.data.iter().find(|sidecar| sidecar.inner.index == hash.index as u64)
            {
                sidecars.push(sidecar.clone());
            }
        });

        timer.observe_duration();
        Ok(sidecars)
    }
}
