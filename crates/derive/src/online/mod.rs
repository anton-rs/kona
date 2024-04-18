//! Contains "online" implementations for providers.

/// Prelude for online providers.
pub(crate) mod prelude {
    pub use super::{
        AlloyChainProvider, AlloyL2ChainProvider, BeaconClient, OnlineBeaconClient,
        OnlineBlobProvider, SimpleSlotDerivation,
    };
}

#[cfg(test)]
#[allow(unreachable_pub)]
pub mod test_utils;

mod beacon_client;
pub use beacon_client::{BeaconClient, OnlineBeaconClient};

mod alloy_providers;
pub use alloy_providers::{AlloyChainProvider, AlloyL2ChainProvider};

mod blob_provider;
pub use blob_provider::{OnlineBlobProvider, SimpleSlotDerivation};

mod utils;
pub(crate) use utils::blobs_from_sidecars;
