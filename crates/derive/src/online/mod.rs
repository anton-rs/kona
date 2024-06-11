//! Contains "online" implementations for providers.

// Re-export types for the online pipeline construction.
pub use crate::{
    pipeline::{DerivationPipeline, PipelineBuilder},
    sources::EthereumDataSource,
    stages::StatefulAttributesBuilder,
    traits::Pipeline,
    types::RollupConfig,
};

mod pipeline;
pub use pipeline::{
    new_online_pipeline, OnlineAttributesBuilder, OnlineAttributesQueue, OnlineDataProvider,
    OnlinePipeline,
};

mod validation;
pub use validation::{OnlineValidator, Validator};

mod beacon_client;
pub use beacon_client::{BeaconClient, OnlineBeaconClient};

mod alloy_providers;
pub use alloy_providers::{AlloyChainProvider, AlloyL2ChainProvider};

mod blob_provider;
pub use blob_provider::{OnlineBlobProvider, SimpleSlotDerivation};

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
