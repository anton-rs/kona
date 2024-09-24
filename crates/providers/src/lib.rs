#![doc = include_str!("../README.md")]
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
    pub use super::*;
    pub use kona_derive::prelude::*;
}

pub mod pipeline;
pub use pipeline::{new_online_pipeline, OnlinePipeline};

pub mod alloy_providers;
pub use alloy_providers::{AlloyChainProvider, AlloyL2ChainProvider};

pub mod beacon_client;
pub use beacon_client::{BeaconClient, OnlineBeaconClient};

pub mod blob_provider;
pub use blob_provider::{
    BlobSidecarProvider, OnlineBlobProvider, OnlineBlobProviderBuilder,
    OnlineBlobProviderWithFallback, SimpleSlotDerivation, SlotDerivation,
};
