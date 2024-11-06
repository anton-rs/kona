//! Test Utilities for Online Providers

use crate::{APIConfigResponse, APIGenesisResponse};
use alloy_eips::eip4844::IndexedBlobHash;
use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_provider::{network::Ethereum, ReqwestProvider};
use alloy_rpc_client::RpcClient;
use alloy_rpc_types_beacon::sidecar::{BeaconBlobBundle, BlobData};
use alloy_transport_http::Http;
use async_trait::async_trait;
use reqwest::Client;

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

/// A mock [crate::BeaconClient] for testing.
#[derive(Debug, Default)]
pub struct MockBeaconClient {
    /// The node version.
    pub node_version: Option<String>,
    /// The config spec.
    pub config_spec: Option<APIConfigResponse>,
    /// The beacon genesis.
    pub beacon_genesis: Option<APIGenesisResponse>,
    /// The blob sidecars.
    pub blob_sidecars: Option<BeaconBlobBundle>,
}

/// A mock beacon client error
#[derive(Debug, derive_more::Display)]
pub enum MockBeaconClientError {
    /// The config spec is not set
    #[display("config_spec not set")]
    ConfigSpecNotSet,
    /// The beacon genesis is not set
    #[display("beacon_genesis not set")]
    BeaconGenesisNotSet,
    /// The blob sidecars are not set
    #[display("blob_sidecars not set")]
    BlobSidecarsNotSet,
}

impl core::error::Error for MockBeaconClientError {}

#[async_trait]
impl crate::BeaconClient for MockBeaconClient {
    type Error = MockBeaconClientError;

    async fn config_spec(&self) -> Result<APIConfigResponse, Self::Error> {
        self.config_spec.clone().ok_or_else(|| MockBeaconClientError::ConfigSpecNotSet)
    }

    async fn beacon_genesis(&self) -> Result<APIGenesisResponse, Self::Error> {
        self.beacon_genesis.clone().ok_or_else(|| MockBeaconClientError::BeaconGenesisNotSet)
    }

    async fn beacon_blob_side_cars(
        &self,
        _: u64,
        _: &[IndexedBlobHash],
    ) -> Result<Vec<BlobData>, Self::Error> {
        self.blob_sidecars
            .clone()
            .ok_or_else(|| MockBeaconClientError::BlobSidecarsNotSet)
            .map(|r| r.data)
    }
}
