//! Contains an online implementation of the [BeaconClient] trait.

use core::fmt::Display;
use alloc::{boxed::Box, format, string::String, vec::Vec, string::ToString};
use async_trait::async_trait;
use reqwest::Client;

use kona_primitives::{
    APIBlobSidecar, APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse,
    IndexedBlobHash,
};

/// The config spec engine api method.
pub(crate) const SPEC_METHOD: &str = "eth/v1/config/spec";

/// The beacon genesis engine api method.
pub(crate) const GENESIS_METHOD: &str = "eth/v1/beacon/genesis";

/// The blob sidecars engine api method prefix.
pub(crate) const SIDECARS_METHOD_PREFIX: &str = "eth/v1/beacon/blob_sidecars";

/// The [BeaconClient] is a thin wrapper around the Beacon API.
#[async_trait]
pub trait BeaconClient {
    /// The error type for [BeaconClient] implementations.
    type Error: Display + ToString;

    /// Returns the config spec.
    async fn config_spec(&self) -> Result<APIConfigResponse, Self::Error>;

    /// Returns the beacon genesis.
    async fn beacon_genesis(&self) -> Result<APIGenesisResponse, Self::Error>;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    async fn beacon_blob_side_cars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<APIBlobSidecar>, Self::Error>;
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
    type Error = reqwest::Error;

    async fn config_spec(&self) -> Result<APIConfigResponse, Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["beacon_client", "config_spec"]);
        crate::timer!(START, PROVIDER_RESPONSE_TIME, &["beacon_client", "config_spec"], timer);
        let first = match self.inner.get(format!("{}/{}", self.base, SPEC_METHOD)).send().await {
            Ok(response) => response,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["beacon_client", "config_spec", "request"]);
                return Err(e);
            }
        };
        match first.json::<APIConfigResponse>().await {
            Ok(response) => Ok(response),
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["beacon_client", "config_spec", "decode"]);
                Err(e)
            }
        }
    }

    async fn beacon_genesis(&self) -> Result<APIGenesisResponse, Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["beacon_client", "beacon_genesis"]);
        crate::timer!(START, PROVIDER_RESPONSE_TIME, &["beacon_client", "beacon_genesis"], timer);
        let first = match self.inner.get(format!("{}/{}", self.base, GENESIS_METHOD)).send().await {
            Ok(response) => response,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["beacon_client", "beacon_genesis", "request"]);
                return Err(e);
            }
        };
        match first.json::<APIGenesisResponse>().await {
            Ok(response) => Ok(response),
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["beacon_client", "beacon_genesis", "decode"]);
                Err(e)
            }
        }
    }

    async fn beacon_blob_side_cars(
        &self,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> Result<Vec<APIBlobSidecar>, Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["beacon_client", "beacon_blob_side_cars"]);
        crate::timer!(
            START,
            PROVIDER_RESPONSE_TIME,
            &["beacon_client", "beacon_blob_side_cars"],
            timer
        );
        let raw_response = match self
            .inner
            .get(format!("{}/{}/{}", self.base, SIDECARS_METHOD_PREFIX, slot))
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["beacon_client", "beacon_blob_side_cars", "request"]
                );
                return Err(e);
            }
        };
        let raw_response = match raw_response.json::<APIGetBlobSidecarsResponse>().await {
            Ok(response) => response,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["beacon_client", "beacon_blob_side_cars", "decode"]);
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

        Ok(sidecars)
    }
}
