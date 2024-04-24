//! Data source

use crate::{
    sources::{BlobSource, CalldataSource, PlasmaSource},
    traits::{AsyncIterator, BlobProvider, ChainProvider},
    types::StageResult,
};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_plasma::traits::{ChainProvider as PlasmaChainProvider, PlasmaInputFetcher};

/// An enum over the various data sources.
#[derive(Debug, Clone)]
pub enum DataSource<CP, B, PCP, PIF, I>
where
    CP: ChainProvider + Send,
    B: BlobProvider + Send,
    PCP: PlasmaChainProvider + Send,
    PIF: PlasmaInputFetcher<PCP> + Send,
    I: Iterator<Item = Bytes> + Send,
{
    /// A calldata source.
    Calldata(CalldataSource<CP>),
    /// A blob source.
    Blob(BlobSource<CP, B>),
    /// A plasma source.
    Plasma(PlasmaSource<PCP, PIF, I>),
}

#[async_trait]
impl<CP, B, PCP, PIF, I> AsyncIterator for DataSource<CP, B, PCP, PIF, I>
where
    CP: ChainProvider + Send,
    B: BlobProvider + Send,
    PCP: PlasmaChainProvider + Send,
    PIF: PlasmaInputFetcher<PCP> + Send,
    I: Iterator<Item = Bytes> + Send,
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
