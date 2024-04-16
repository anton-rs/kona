//! Test Utilities for Online Providers

use super::BeaconClient;
use crate::types::{
    APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse, IndexedBlobHash,
};
use alloc::{boxed::Box, string::String, vec::Vec};
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_provider::{network::Ethereum, ReqwestProvider};
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use async_trait::async_trait;
use reqwest::Client;

pub(crate) fn spawn_anvil() -> (ReqwestProvider<Ethereum>, AnvilInstance) {
    let anvil = Anvil::new().try_spawn().expect("could not spawn anvil");
    (anvil_http_provider(&anvil), anvil)
}

pub(crate) fn anvil_http_provider(anvil: &AnvilInstance) -> ReqwestProvider<Ethereum> {
    http_provider(&anvil.endpoint())
}

pub(crate) fn http_provider(url: &str) -> ReqwestProvider<Ethereum> {
    let url = url.parse().unwrap();
    let http = Http::<Client>::new(url);
    ReqwestProvider::new(RpcClient::new(http, true))
}

/// A mock [BeaconClient] for testing.
#[derive(Debug, Default)]
pub struct MockBeaconClient {
    /// The node version.
    pub node_version: Option<String>,
    /// The config spec.
    pub config_spec: Option<APIConfigResponse>,
    /// The beacon genesis.
    pub beacon_genesis: Option<APIGenesisResponse>,
    /// The blob sidecars.
    pub blob_sidecars: Option<APIGetBlobSidecarsResponse>,
}

#[async_trait]
impl BeaconClient for MockBeaconClient {
    async fn node_version(&self) -> anyhow::Result<String> {
        self.node_version.clone().ok_or_else(|| anyhow::anyhow!("node_version not set"))
    }

    async fn config_spec(&self) -> anyhow::Result<APIConfigResponse> {
        self.config_spec.clone().ok_or_else(|| anyhow::anyhow!("config_spec not set"))
    }

    async fn beacon_genesis(&self) -> anyhow::Result<APIGenesisResponse> {
        self.beacon_genesis.clone().ok_or_else(|| anyhow::anyhow!("beacon_genesis not set"))
    }

    async fn beacon_blob_side_cars(
        &self,
        _fetch_all_sidecars: bool,
        _slot: u64,
        _hashes: Vec<IndexedBlobHash>,
    ) -> anyhow::Result<APIGetBlobSidecarsResponse> {
        self.blob_sidecars.clone().ok_or_else(|| anyhow::anyhow!("blob_sidecars not set"))
    }
}
