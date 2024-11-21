//! Contains utility functions and helpers for the host program.

use alloy_primitives::{hex, Bytes};
use alloy_provider::ReqwestProvider;
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use anyhow::{anyhow, Result};
use kona_proof::HintType;
use reqwest::Client;

/// Parses a hint from a string.
///
/// Hints are of the format `<hint_type> <hint_data>`, where `<hint_type>` is a string that
/// represents the type of hint, and `<hint_data>` is the data associated with the hint
/// (bytes encoded as hex UTF-8).
pub(crate) fn parse_hint(s: &str) -> Result<(HintType, Bytes)> {
    let mut parts = s.split(' ').collect::<Vec<_>>();

    if parts.len() != 2 {
        anyhow::bail!("Invalid hint format: {}", s);
    }

    let hint_type = HintType::try_from(parts.remove(0))?;
    let hint_data = hex::decode(parts.remove(0)).map_err(|e| anyhow!(e))?.into();

    Ok((hint_type, hint_data))
}

/// Returns an HTTP provider for the given URL.
pub(crate) fn http_provider(url: &str) -> ReqwestProvider {
    let url = url.parse().unwrap();
    let http = Http::<Client>::new(url);
    ReqwestProvider::new(RpcClient::new(http, true))
}
