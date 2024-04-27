//! Contains an online implementation of the [BeaconClient] trait.

use crate::types::{
    APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse, IndexedBlobHash,
};
use alloc::{boxed::Box, string::String};
use alloy_provider::Provider;
use alloy_transport_http::Http;
use async_trait::async_trait;
use reqwest::Client;

/// The node version engine api method.
pub(crate) const VERSION_METHOD: &str = "eth/v1/node/version";

/// The config spec engine api method.
pub(crate) const SPEC_METHOD: &str = "eth/v1/config/spec";

/// The beacon genesis engine api method.
pub(crate) const GENESIS_METHOD: &str = "eth/v1/beacon/genesis";

/// The blob sidecars engine api method prefix.
pub(crate) const SIDECARS_METHOD_PREFIX: &str = "eth/v1/beacon/blob_sidecars/";

/// The [BeaconClient] is a thin wrapper around the Beacon API.
#[async_trait]
pub trait BeaconClient {
    /// Returns the node version.
    async fn node_version(&self) -> anyhow::Result<String>;

    /// Returns the config spec.
    async fn config_spec(&self) -> anyhow::Result<APIConfigResponse>;

    /// Returns the beacon genesis.
    async fn beacon_genesis(&self) -> anyhow::Result<APIGenesisResponse>;

    /// Fetches blob sidecars that were confirmed in the specified L1 block with the given indexed
    /// hashes. Order of the returned sidecars is guaranteed to be that of the hashes. Blob data is
    /// not checked for validity.
    async fn beacon_blob_side_cars(
        &self,
        fetch_all_sidecars: bool,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> anyhow::Result<APIGetBlobSidecarsResponse>;
}

/// An online implementation of the [BeaconClient] trait.
#[derive(Debug, Clone)]
pub struct OnlineBeaconClient<T: Provider<Http<Client>>> {
    /// The inner Ethereum JSON-RPC provider.
    inner: T,
}

impl<T: Provider<Http<Client>>> OnlineBeaconClient<T> {
    /// Creates a new instance of the [OnlineBeaconClient].
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<T: Provider<Http<Client>> + Send> BeaconClient for OnlineBeaconClient<T> {
    async fn node_version(&self) -> anyhow::Result<String> {
        self.inner.client().request(VERSION_METHOD, ()).await.map_err(|e| anyhow::anyhow!(e))
    }

    async fn config_spec(&self) -> anyhow::Result<APIConfigResponse> {
        self.inner.client().request(SPEC_METHOD, ()).await.map_err(|e| anyhow::anyhow!(e))
    }

    async fn beacon_genesis(&self) -> anyhow::Result<APIGenesisResponse> {
        self.inner.client().request(GENESIS_METHOD, ()).await.map_err(|e| anyhow::anyhow!(e))
    }

    async fn beacon_blob_side_cars(
        &self,
        fetch_all_sidecars: bool,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> anyhow::Result<APIGetBlobSidecarsResponse> {
        let method = alloc::format!("{}{}", SIDECARS_METHOD_PREFIX, slot);
        self.inner
            .client()
            .request(method, (fetch_all_sidecars, hashes))
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
