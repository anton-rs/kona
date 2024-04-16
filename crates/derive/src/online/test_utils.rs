//! Test Utilities for Online Providers

use alloy_node_bindings::{Anvil, AnvilInstance};
use alloy_provider::{network::Ethereum, ReqwestProvider};
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
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
