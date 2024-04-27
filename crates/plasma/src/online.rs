//! Module contains an online implementation of the Plasma Input Fetcher.

use crate::{FinalizedHeadSignal, PlasmaError, PlasmaInputFetcher};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use alloy_provider::ReqwestProvider;
use async_trait::async_trait;
use kona_primitives::{
    block::{BlockID, BlockInfo},
    system_config::SystemConfig,
};
use kona_providers::AlloyChainProvider;

type OnlineCP = AlloyChainProvider<ReqwestProvider>;

/// An Online Plasma Input Fetcher.
#[derive(Debug, Clone)]
pub struct OnlinePlasmaInputFetcher {}

#[async_trait]
impl PlasmaInputFetcher<OnlineCP> for OnlinePlasmaInputFetcher {
    async fn get_input(
        &mut self,
        _fetcher: &OnlineCP,
        _commitment: Bytes,
        _block: BlockID,
    ) -> Option<Result<Bytes, PlasmaError>> {
        unimplemented!()
    }

    async fn advance_l1_origin(
        &mut self,
        _fetcher: &OnlineCP,
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
