//! Plasma Data Source

use crate::traits::AsyncIterator;
use crate::types::StageResult;
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;

/// A plasma data iterator.
#[derive(Debug, Clone, Default)]
pub struct PlasmaSource {}

impl PlasmaSource {
    /// Instantiates a new plasma data source.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AsyncIterator for PlasmaSource {
    type Item = StageResult<Bytes>;

    async fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
