//! Data source

use crate::{
    sources::{BlobSource, CalldataSource, PlasmaSource},
    traits::{AsyncIterator, BlobProvider},
    types::StageResult,
};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_plasma::traits::PlasmaInputFetcher;
use kona_providers::ChainProvider;

/// An enum over the various data sources.
#[derive(Debug, Clone)]
pub enum DataSource<CP, B, PIF>
where
    CP: ChainProvider + Send + Clone,
    B: BlobProvider + Send + Clone,
    PIF: PlasmaInputFetcher<CP> + Send,
{
    /// A calldata source.
    Calldata(CalldataSource<CP>),
    /// A blob source.
    Blob(BlobSource<CP, B>),
    /// A plasma source.
    Plasma(PlasmaSource<CP, B, PIF>),
}

#[async_trait]
impl<CP, B, PIF> AsyncIterator for DataSource<CP, B, PIF>
where
    CP: ChainProvider + Send + Clone,
    B: BlobProvider + Send + Clone,
    PIF: PlasmaInputFetcher<CP> + Send,
{
    type Item = Bytes;

    async fn next(&mut self) -> Option<StageResult<Self::Item>> {
        match self {
            DataSource::Calldata(c) => c.next().await,
            DataSource::Blob(b) => b.next().await,
            DataSource::Plasma(p) => p.next().await,
        }
    }
}
