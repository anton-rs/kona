//! This module contains traits for the plasma extension of the derivation pipeline.

use crate::types::{FinalizedHeadSignal, PlasmaError};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_derive::traits::ChainProvider;
use kona_primitives::{BlockID, BlockInfo, SystemConfig};

/// A plasma input fetcher.
#[async_trait]
pub trait PlasmaInputFetcher<CP: ChainProvider + Send> {
    /// Get the input for the given commitment at the given block number from the DA storage
    /// service.
    async fn get_input(
        &mut self,
        fetcher: &CP,
        commitment: Bytes,
        block: BlockID,
    ) -> Option<Result<Bytes, PlasmaError>>;

    /// Advance the L1 origin to the given block number, syncing the DA challenge events.
    async fn advance_l1_origin(
        &mut self,
        fetcher: &CP,
        block: BlockID,
    ) -> Option<Result<(), PlasmaError>>;

    /// Reset the challenge origin in case of L1 reorg.
    async fn reset(
        &mut self,
        block_number: BlockInfo,
        cfg: SystemConfig,
    ) -> Option<Result<(), PlasmaError>>;

    /// Notify L1 finalized head so plasma finality is always behind L1.
    async fn finalize(&mut self, block_number: BlockInfo) -> Option<Result<(), PlasmaError>>;

    /// Set the engine finalization signal callback.
    fn on_finalized_head_signal(&mut self, callback: FinalizedHeadSignal);
}

#[async_trait]
impl<CP: ChainProvider + Send> PlasmaInputFetcher<CP> for () {
    /// Get the input for the given commitment at the given block number from the DA storage
    /// service.
    async fn get_input(
        &mut self,
        _fetcher: &CP,
        _commitment: Bytes,
        _block: BlockID,
    ) -> Option<Result<Bytes, PlasmaError>> {
        unimplemented!()
    }

    /// Advance the L1 origin to the given block number, syncing the DA challenge events.
    async fn advance_l1_origin(
        &mut self,
        _fetcher: &CP,
        _block: BlockID,
    ) -> Option<Result<(), PlasmaError>> {
        unimplemented!()
    }

    /// Reset the challenge origin in case of L1 reorg.
    async fn reset(
        &mut self,
        _block_number: BlockInfo,
        _cfg: SystemConfig,
    ) -> Option<Result<(), PlasmaError>> {
        unimplemented!()
    }

    /// Notify L1 finalized head so plasma finality is always behind L1.
    async fn finalize(&mut self, _block_number: BlockInfo) -> Option<Result<(), PlasmaError>> {
        unimplemented!()
    }

    /// Set the engine finalization signal callback.
    fn on_finalized_head_signal(&mut self, _callback: FinalizedHeadSignal) {
        unimplemented!()
    }
}
