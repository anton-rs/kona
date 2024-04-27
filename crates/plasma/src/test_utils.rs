//! Test utilities for the Plasma crate.

use crate::{
    traits::PlasmaInputFetcher,
    types::{FinalizedHeadSignal, PlasmaError},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::Bytes;
use anyhow::Result;
use async_trait::async_trait;
use kona_primitives::{
    block::{BlockID, BlockInfo},
    system_config::SystemConfig,
};
use kona_providers::test_utils::TestChainProvider;

/// A mock plasma input fetcher for testing.
#[derive(Debug, Clone, Default)]
pub struct TestPlasmaInputFetcher {
    /// Inputs to return.
    pub inputs: Vec<Result<Bytes, PlasmaError>>,
    /// Advance L1 origin results.
    pub advances: Vec<Result<(), PlasmaError>>,
    /// Reset results.
    pub resets: Vec<Result<(), PlasmaError>>,
}

#[async_trait]
impl PlasmaInputFetcher<TestChainProvider> for TestPlasmaInputFetcher {
    async fn get_input(
        &mut self,
        _fetcher: &TestChainProvider,
        _commitment: Bytes,
        _block: BlockID,
    ) -> Option<Result<Bytes, PlasmaError>> {
        self.inputs.pop()
    }

    async fn advance_l1_origin(
        &mut self,
        _fetcher: &TestChainProvider,
        _block: BlockID,
    ) -> Option<Result<(), PlasmaError>> {
        self.advances.pop()
    }

    async fn reset(
        &mut self,
        _block_number: BlockInfo,
        _cfg: SystemConfig,
    ) -> Option<Result<(), PlasmaError>> {
        self.resets.pop()
    }

    async fn finalize(&mut self, _block_number: BlockInfo) -> Option<Result<(), PlasmaError>> {
        None
    }

    fn on_finalized_head_signal(&mut self, _block_number: FinalizedHeadSignal) {}
}
