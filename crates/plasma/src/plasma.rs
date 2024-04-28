//! Contains the [PlasmaDataSource], which is a concrete implementation of the
//! [DataAvailabilityProvider] trait for the Plasma protocol. Used as an adapter to the
//! [kona_derive] crate's derivation pipeline construction.
//!
//! [DataAvailabilityProvider]: kona_derive::traits::DataAvailabilityProvider

use crate::{source::PlasmaSource, traits::PlasmaInputFetcher};
use alloc::{boxed::Box, fmt::Debug};
use alloy_primitives::{Address, Bytes};
use anyhow::Result;
use async_trait::async_trait;
use kona_derive::{
    traits::{ChainProvider, DataAvailabilityProvider},
    types::{BlockInfo, RollupConfig},
};
use kona_primitives::BlockID;

/// A factory for creating an Ethereum data source provider.
#[derive(Debug, Clone, Copy)]
pub struct PlasmaDataSource<C, PIF, I>
where
    C: ChainProvider + Send + Clone,
    PIF: PlasmaInputFetcher<C> + Clone,
    I: Iterator<Item = Bytes> + Send + Clone,
{
    /// The chain provider to use for the factory.
    pub chain_provider: C,
    /// The plasma iterator.
    pub plasma_source: I,
    /// The plasma input fetcher.
    pub plasma_input_fetcher: PIF,
    /// The L1 Signer.
    pub signer: Address,
}

impl<C, PIF, I> PlasmaDataSource<C, PIF, I>
where
    C: ChainProvider + Send + Clone + Debug,
    PIF: PlasmaInputFetcher<C> + Clone,
    I: Iterator<Item = Bytes> + Send + Clone,
{
    /// Creates a new factory.
    pub fn new(provider: C, pif: PIF, s: I, cfg: &RollupConfig) -> Self {
        Self {
            chain_provider: provider,
            plasma_source: s,
            plasma_input_fetcher: pif,
            signer: cfg.genesis.system_config.batcher_addr,
        }
    }
}

#[async_trait]
impl<C, PIF, I> DataAvailabilityProvider for PlasmaDataSource<C, PIF, I>
where
    C: ChainProvider + Send + Clone + Debug + Sync,
    PIF: PlasmaInputFetcher<C> + Clone + Debug + Send + Sync,
    I: Iterator<Item = Bytes> + Send + Clone + Debug + Sync,
{
    type Item = Bytes;
    type DataIter = PlasmaSource<C, PIF, I>;

    async fn open_data(&self, block_ref: &BlockInfo, _: Address) -> Result<Self::DataIter> {
        let id = BlockID { hash: block_ref.hash, number: block_ref.number };
        Ok(PlasmaSource::new(
            self.chain_provider.clone(),
            self.plasma_input_fetcher.clone(),
            self.plasma_source.clone(),
            id,
        ))
    }
}
