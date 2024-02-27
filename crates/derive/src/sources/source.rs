//! Data source

use crate::sources::{BlobSource, CalldataSource, PlasmaSource};
use crate::traits::ChainProvider;
use crate::types::StageResult;
use alloy_primitives::Bytes;
use async_iterator::Iterator;

/// An enum over the various data sources.
#[derive(Debug, Clone)]
pub enum DataSource<CP: ChainProvider> {
    /// A calldata source.
    Calldata(CalldataSource<CP>),
    /// A blob source.
    Blob(BlobSource),
    /// A plasma source.
    Plasma(PlasmaSource),
}

impl<CP: ChainProvider> Iterator for DataSource<CP> {
    type Item = StageResult<Bytes>;

    async fn next(&mut self) -> Option<Self::Item> {
        match self {
            DataSource::Calldata(c) => c.next().await,
            DataSource::Blob(b) => b.next().await,
            DataSource::Plasma(p) => p.next().await,
        }
    }
}
