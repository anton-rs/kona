//! Blob Data Source

use crate::traits::AsyncIterator;
use crate::types::StageResult;
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;

/// A data iterator that reads from a blob.
#[derive(Debug, Clone, Default)]
pub struct BlobSource {}

impl BlobSource {
    /// Creates a new blob data source.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AsyncIterator for BlobSource {
    type Item = StageResult<Bytes>;

    async fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
