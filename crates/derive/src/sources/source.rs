//! Data source

use crate::{
    sources::{BlobSource, CalldataSource, PlasmaSource},
    traits::{AsyncIterator, BlobProvider, ChainProvider, PlasmaInputFetcher},
    types::StageResult,
};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;

/// An enum over the various data sources.
#[derive(Debug, Clone)]
pub enum DataSource<CP, B, PIF, I>
where
    CP: ChainProvider + Send,
    B: BlobProvider + Send,
    PIF: PlasmaInputFetcher<CP> + Send,
    I: Iterator<Item = Bytes> + Send,
{
    /// A calldata source.
    Calldata(CalldataSource<CP>),
    /// A blob source.
    Blob(BlobSource<CP, B>),
    /// A plasma source.
    Plasma(PlasmaSource<CP, PIF, I>),
}

#[async_trait]
impl<CP, B, PIF, I> AsyncIterator for DataSource<CP, B, PIF, I>
where
    CP: ChainProvider + Send,
    B: BlobProvider + Send,
    PIF: PlasmaInputFetcher<CP> + Send,
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
