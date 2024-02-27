//! Data source

use crate::sources::{BlobSource, CalldataSource, PlasmaSource};
use crate::traits::{AsyncIterator, BlobProvider, ChainProvider};
use crate::types::StageResult;
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;

/// An enum over the various data sources.
#[derive(Debug, Clone)]
pub enum DataSource<CP, B>
where
    CP: ChainProvider + Send,
    B: BlobProvider + Send,
{
    /// A calldata source.
    Calldata(CalldataSource<CP>),
    /// A blob source.
    Blob(BlobSource<CP, B>),
    /// A plasma source.
    Plasma(PlasmaSource<CP>),
}

#[async_trait]
impl<CP, B> AsyncIterator for DataSource<CP, B>
where
    CP: ChainProvider + Send,
    B: BlobProvider + Send,
{
    type Item = StageResult<Bytes>;

    async fn next(&mut self) -> Option<Self::Item> {
        match self {
            DataSource::Calldata(c) => c.next().await,
            DataSource::Blob(b) => b.next().await,
            DataSource::Plasma(p) => p.next().await,
        }
    }
}
