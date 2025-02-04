//! Ethereum utilities for the host binary.

use alloy_provider::{Network, RootProvider};
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use reqwest::Client;

mod precompiles;
pub(crate) use precompiles::execute;

/// Returns an HTTP provider for the given URL.
pub fn http_provider<N: Network>(url: &str) -> RootProvider<N> {
    let url = url.parse().unwrap();
    let http = Http::<Client>::new(url);
    RootProvider::new(RpcClient::new(http, true))
}
