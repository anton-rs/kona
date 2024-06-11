//! Module contains an online implementation of the Plasma Input Fetcher.

use crate::{
    traits::PlasmaInputFetcher,
    types::{FinalizedHeadSignal, PlasmaError},
};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_derive::online::AlloyChainProvider;
use kona_primitives::{
    block::{BlockID, BlockInfo},
    system_config::SystemConfig,
};

/// An Online Plasma Input Fetcher.
#[derive(Debug, Clone)]
pub struct OnlinePlasmaInputFetcher {}

#[async_trait]
impl PlasmaInputFetcher<AlloyChainProvider> for OnlinePlasmaInputFetcher {
    async fn get_input(
        &mut self,
        _fetcher: &AlloyChainProvider,
        _commitment: Bytes,
        _block: BlockID,
    ) -> Option<Result<Bytes, PlasmaError>> {
        unimplemented!()
    }

    async fn advance_l1_origin(
        &mut self,
        _fetcher: &AlloyChainProvider,
        _block: BlockID,
    ) -> Option<Result<(), PlasmaError>> {
        unimplemented!()
    }

    async fn reset(
        &mut self,
        _block_number: BlockInfo,
        _cfg: SystemConfig,
    ) -> Option<Result<(), PlasmaError>> {
        unimplemented!()
    }

    async fn finalize(&mut self, _block_number: BlockInfo) -> Option<Result<(), PlasmaError>> {
        unimplemented!()
    }

    fn on_finalized_head_signal(&mut self, _callback: FinalizedHeadSignal) {
        unimplemented!()
    }
}
