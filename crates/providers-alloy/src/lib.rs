#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod macros;

#[cfg(feature = "metrics")]
pub mod metrics;
#[cfg(feature = "metrics")]
pub use metrics::*;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
#[cfg(any(test, feature = "test-utils"))]
pub use test_utils::*;

/// Re-export commonly used types and traits.
pub mod prelude {
    pub use super::{
        alloy_providers::{AlloyChainProvider, AlloyL2ChainProvider},
        beacon_client::{BeaconClient, OnlineBeaconClient},
        blob_provider::{
            BlobSidecarProvider, OnlineBlobProvider, OnlineBlobProviderBuilder,
            OnlineBlobProviderWithFallback,
        },
        pipeline::{new_online_pipeline, OnlinePipeline},
    };
    pub use kona_derive::prelude::*;
}

pub mod pipeline;
pub use pipeline::{new_online_pipeline, OnlinePipeline};

pub mod alloy_providers;
pub use alloy_providers::{AlloyChainProvider, AlloyL2ChainProvider};

pub mod beacon_client;
pub use beacon_client::{
    APIBlobSidecar, APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse,
    BeaconClient, OnlineBeaconClient,
};

pub mod blob_provider;
pub use blob_provider::{
    BlobSidecarProvider, OnlineBlobProvider, OnlineBlobProviderBuilder,
    OnlineBlobProviderWithFallback,
};
