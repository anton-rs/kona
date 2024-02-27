//! Data source

use crate::sources::{BlobSource, CalldataSource, PlasmaSource};
use crate::traits::{AsyncIterator, ChainProvider};
use crate::types::StageResult;
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;

/// An enum over the various data sources.
#[derive(Debug, Clone)]
pub enum DataSource<CP: ChainProvider + Send> {
    /// A calldata source.
    Calldata(CalldataSource<CP>),
    /// A blob source.
    Blob(BlobSource),
    /// A plasma source.
    Plasma(PlasmaSource),
}

#[async_trait]
impl<CP: ChainProvider + Send> AsyncIterator for DataSource<CP> {
    type Item = StageResult<Bytes>;

    async fn next(&mut self) -> Option<Self::Item> {
        match self {
            DataSource::Calldata(c) => c.next().await,
            DataSource::Blob(b) => b.next().await,
            DataSource::Plasma(p) => p.next().await,
        }
    }
}
