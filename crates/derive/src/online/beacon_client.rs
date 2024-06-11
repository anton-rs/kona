//! Contains an online implementation of the [BeaconClient] trait.

use crate::types::{
    APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse, IndexedBlobHash,
};
use alloc::{boxed::Box, string::String};
use alloy_provider::{Provider, ReqwestProvider};
use alloy_transport::TransportResult;
use async_trait::async_trait;

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
pub struct OnlineBeaconClient {
    /// The inner Ethereum JSON-RPC provider.
    inner: ReqwestProvider,
}

impl OnlineBeaconClient {
    /// Creates a new instance of the [OnlineBeaconClient].
    pub fn new(inner: ReqwestProvider) -> Self {
        Self { inner }
    }

    /// Creates a new [OnlineBeaconClient] from the provided [reqwest::Url].
    pub fn new_http(url: reqwest::Url) -> Self {
        let inner = ReqwestProvider::new_http(url);
        Self::new(inner)
    }
}

#[async_trait]
impl BeaconClient for OnlineBeaconClient {
    async fn node_version(&self) -> anyhow::Result<String> {
        let res: TransportResult<String> = self.inner.raw_request(VERSION_METHOD.into(), ()).await;
        res.map_err(|e| anyhow::anyhow!(e))
    }

    async fn config_spec(&self) -> anyhow::Result<APIConfigResponse> {
        let res: TransportResult<APIConfigResponse> =
            self.inner.raw_request(SPEC_METHOD.into(), ()).await;
        res.map_err(|e| anyhow::anyhow!(e))
    }

    async fn beacon_genesis(&self) -> anyhow::Result<APIGenesisResponse> {
        let res: TransportResult<APIGenesisResponse> =
            self.inner.raw_request(GENESIS_METHOD.into(), ()).await;
        res.map_err(|e| anyhow::anyhow!(e))
    }

    async fn beacon_blob_side_cars(
        &self,
        fetch_all_sidecars: bool,
        slot: u64,
        hashes: &[IndexedBlobHash],
    ) -> anyhow::Result<APIGetBlobSidecarsResponse> {
        let method = alloc::format!("{}{}", SIDECARS_METHOD_PREFIX, slot);
        let res: TransportResult<APIGetBlobSidecarsResponse> =
            self.inner.raw_request(method.into(), (fetch_all_sidecars, hashes)).await;
        res.map_err(|e| anyhow::anyhow!(e))
    }
}
