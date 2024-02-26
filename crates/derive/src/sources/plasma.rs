//! Plasma Data Source

use crate::traits::DataIter;
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

impl<T: Into<Bytes>> DataIter<T> for PlasmaSource {
    fn next(&mut self) -> StageResult<T> {
        unimplemented!()
    }
}
