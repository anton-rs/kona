//! Blob Data Source

use crate::types::StageResult;
use alloy_primitives::Bytes;
use async_iterator::Iterator;

/// A data iterator that reads from a blob.
#[derive(Debug, Clone, Default)]
pub struct BlobSource {}

impl BlobSource {
    /// Creates a new blob data source.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Iterator for BlobSource {
    type Item = StageResult<Bytes>;

    async fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
