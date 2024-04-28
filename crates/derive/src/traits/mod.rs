//! This module contains all of the traits describing functionality of portions of the derivation
//! pipeline.

mod data_sources;
pub use data_sources::*;

mod providers;
pub use providers::{ChainProvider, L2ChainProvider};

mod plasma;
pub use plasma::PlasmaInputFetcher;

mod stages;
pub use stages::{OriginAdvancer, OriginProvider, PreviousStage, ResettableStage};

mod ecrecover;
pub use ecrecover::SignedRecoverable;

#[cfg(test)]
pub mod test_utils;
