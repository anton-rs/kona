//! Plasma Data Source

use crate::types::StageResult;
use alloy_primitives::Bytes;

/// A plasma data iterator.
#[derive(Debug, Clone)]
pub struct PlasmaSource {}

impl PlasmaSource {
    /// Instantiates a new plasma data source.
    pub fn new() -> Self {
        Self {}
    }
}

impl Iterator for PlasmaSource {
    type Item = StageResult<Bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
