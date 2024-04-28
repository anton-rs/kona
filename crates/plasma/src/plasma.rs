//! Contains the [PlasmaDataSource], which is a concrete implementation of the
//! [DataAvailabilityProvider] trait for the Plasma source. Used as an adapter to the
//! [kona_derive] crate's derivation pipeline construction.

use crate::{source::PlasmaSource, traits::PlasmaInputFetcher};
use alloc::{boxed::Box, fmt::Debug};
use alloy_primitives::{Address, Bytes};
use anyhow::Result;
use async_trait::async_trait;
use kona_derive::traits::{ChainProvider, DataAvailabilityProvider};
use kona_primitives::BlockInfo;

/// The plasma data source implements the [DataAvailabilityProvider] trait for the Plasma source.
#[derive(Debug, Clone, Copy)]
pub struct PlasmaDataSource<C, F, I>
where
    C: ChainProvider + Send + Clone,
    F: PlasmaInputFetcher<C> + Clone,
    I: Iterator<Item = Bytes> + Send + Clone,
{
    /// The chain provider.
    pub chain_provider: C,
    /// The plasma input fetcher.
    pub plasma_input_fetcher: F,
    /// The plasma iterator.
    pub plasma_source: I,
}

impl<C, F, I> PlasmaDataSource<C, F, I>
where
    C: ChainProvider + Send + Clone + Debug,
    F: PlasmaInputFetcher<C> + Clone,
    I: Iterator<Item = Bytes> + Send + Clone,
{
    /// Creates a new [PlasmaDataSource] from the given providers.
    pub fn new(chain_provider: C, plasma_input_fetcher: F, plasma_source: I) -> Self {
        Self { chain_provider, plasma_input_fetcher, plasma_source }
    }
}

#[async_trait]
impl<C, F, I> DataAvailabilityProvider for PlasmaDataSource<C, F, I>
where
    C: ChainProvider + Send + Clone + Debug + Sync,
    F: PlasmaInputFetcher<C> + Clone + Debug + Send + Sync,
    I: Iterator<Item = Bytes> + Send + Clone + Debug + Sync,
{
    type Item = Bytes;
    type DataIter = PlasmaSource<C, F, I>;

    async fn open_data(&self, block_ref: &BlockInfo, _: Address) -> Result<Self::DataIter> {
        Ok(PlasmaSource::new(
            self.chain_provider.clone(),
            self.plasma_input_fetcher.clone(),
            self.plasma_source.clone(),
            block_ref.id(),
        ))
    }
}
