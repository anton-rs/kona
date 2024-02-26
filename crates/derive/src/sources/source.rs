//! Data source

use crate::sources::{BlobSource, CalldataSource, PlasmaSource};
use crate::traits::ChainProvider;
use crate::types::StageResult;
use alloy_primitives::Bytes;

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

impl<CP: ChainProvider, T: Into<Bytes>> Iterator for DataSource<CP> {
    type Item = StageResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            DataSource::Calldata(c) => c.next().ok(),
            DataSource::Blob(b) => b.next().ok(),
            DataSource::Plasma(p) => p.next().ok(),
        }
    }
}
