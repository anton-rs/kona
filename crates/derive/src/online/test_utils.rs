//! Test Utilities for Online Providers

use alloc::{boxed::Box, string::String, vec::Vec};
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_provider::{network::Ethereum, ReqwestProvider};
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_primitives::{
    APIBlobSidecar, APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse,
    IndexedBlobHash,
};
use reqwest::Client;

use super::BeaconClient;

/// Spawns an Anvil instance and returns a provider and the instance.
pub fn spawn_anvil() -> (ReqwestProvider<Ethereum>, AnvilInstance) {
    let anvil = Anvil::new().try_spawn().expect("could not spawn anvil");
    (anvil_http_provider(&anvil), anvil)
}

/// Returns an Anvil HTTP provider wrapping the given [AnvilInstance].
pub fn anvil_http_provider(anvil: &AnvilInstance) -> ReqwestProvider<Ethereum> {
    http_provider(&anvil.endpoint())
}

/// Returns an HTTP provider for the given URL.
pub fn http_provider(url: &str) -> ReqwestProvider<Ethereum> {
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
    type Error = anyhow::Error;

    async fn config_spec(&self) -> Result<APIConfigResponse> {
        self.config_spec.clone().ok_or_else(|| anyhow!("config_spec not set"))
    }

    async fn beacon_genesis(&self) -> Result<APIGenesisResponse> {
        self.beacon_genesis.clone().ok_or_else(|| anyhow!("beacon_genesis not set"))
    }

    async fn beacon_blob_side_cars(
        &self,
        _: u64,
        _: &[IndexedBlobHash],
    ) -> Result<Vec<APIBlobSidecar>> {
        self.blob_sidecars.clone().ok_or_else(|| anyhow!("blob_sidecars not set")).map(|r| r.data)
    }
}
