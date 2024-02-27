//! Plasma Data Source

use crate::types::StageResult;
use alloy_primitives::Bytes;
use async_iterator::Iterator;

/// A plasma data iterator.
#[derive(Debug, Clone, Default)]
pub struct PlasmaSource {}

impl PlasmaSource {
    /// Instantiates a new plasma data source.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Iterator for PlasmaSource {
    type Item = StageResult<Bytes>;

    async fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
