//! Contains an online implementation of the `BeaconClient` trait.

use alloy_eips::{eip1898::NumHash, eip4844::BlobTransactionSidecarItem};
use alloy_primitives::FixedBytes;
use async_trait::async_trait;
use reqwest::Client;

/// The config spec engine api method.
pub(crate) const SPEC_METHOD: &str = "eth/v1/config/spec";

/// The beacon genesis engine api method.
pub(crate) const GENESIS_METHOD: &str = "eth/v1/beacon/genesis";

/// The blob sidecars engine api method prefix.
pub(crate) const SIDECARS_METHOD_PREFIX: &str = "eth/v1/beacon/blob_sidecars";

/// An API blob sidecar.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct APIBlobSidecar {
    /// The inner blob sidecar.
    #[serde(flatten)]
    pub inner: BlobTransactionSidecarItem,
    /// The signed block header.
    #[serde(rename = "signed_block_header")]
    pub signed_block_header: SignedBeaconBlockHeader,
    // The inclusion-proof of the blob-sidecar into the beacon-block is ignored,
    // since we verify blobs by their versioned hashes against the execution-layer block instead.
}

/// A signed beacon block header.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignedBeaconBlockHeader {
    /// The message.
    pub message: BeaconBlockHeader,
    // The signature is ignored, since we verify blobs against EL versioned-hashes
}

/// A beacon block header.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BeaconBlockHeader {
    /// The slot.
    #[serde(with = "alloy_serde::quantity")]
    pub slot: u64,
    /// The proposer index.
    #[serde(with = "alloy_serde::quantity")]
    pub proposer_index: u64,
    /// The parent root.
    #[serde(rename = "parent_root")]
    pub parent_root: FixedBytes<32>,
    /// The state root.
    #[serde(rename = "state_root")]
    pub state_root: FixedBytes<32>,
    /// The body root.
    #[serde(rename = "body_root")]
    pub body_root: FixedBytes<32>,
}

/// An response for fetching blob sidecars.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct APIGetBlobSidecarsResponse {
    /// The data.
    pub data: Vec<APIBlobSidecar>,
}

/// A reduced genesis data.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReducedGenesisData {
    /// The genesis time.
    #[serde(rename = "genesis_time")]
    #[serde(with = "alloy_serde::quantity")]
    pub genesis_time: u64,
}

/// An API genesis response.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct APIGenesisResponse {
    /// The data.
    pub data: ReducedGenesisData,
}

/// A reduced config data.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReducedConfigData {
    /// The seconds per slot.
    #[serde(rename = "SECONDS_PER_SLOT")]
    #[serde(with = "alloy_serde::quantity")]
    pub seconds_per_slot: u64,
}

/// An API config response.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct APIConfigResponse {
    /// The data.
    pub data: ReducedConfigData,
}

impl APIConfigResponse {
    /// Creates a new API config response.
    pub const fn new(seconds_per_slot: u64) -> Self {
        Self { data: ReducedConfigData { seconds_per_slot } }
    }
}

/// An API version response.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct APIVersionResponse {
    /// The data.
    pub data: VersionInformation,
}

/// Version information.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VersionInformation {
    /// The version.
    pub version: String,
}

impl APIGenesisResponse {
    /// Creates a new API genesis response.
    pub const fn new(genesis_time: u64) -> Self {
        Self { data: ReducedGenesisData { genesis_time } }
    }
}

impl Clone for APIGetBlobSidecarsResponse {
    fn clone(&self) -> Self {
        let mut data = Vec::with_capacity(self.data.len());
        for sidecar in &self.data {
            data.push(sidecar.clone());
        }
        Self { data }
    }
}

/// The [BeaconClient] is a thin wrapper around the Beacon API.
#[async_trait]
pub trait BeaconClient {
    /// The error type for [BeaconClient] implementations.
    type Error: std::fmt::Display + ToString;

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
        hashes: &[NumHash],
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
        hashes: &[NumHash],
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
                raw_response.data.iter().find(|sidecar| sidecar.inner.index == hash.number)
            {
                sidecars.push(sidecar.clone());
            }
        });

        Ok(sidecars)
    }
}
