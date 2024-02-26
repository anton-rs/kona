//! Blob Data Source

use crate::types::StageResult;
use alloy_primitives::Bytes;

/// A data iterator that reads from a blob.
#[derive(Debug, Clone)]
pub struct BlobSource {}

impl BlobSource {
    /// Creates a new blob data source.
    pub fn new() -> Self {
        Self {}
    }
}

impl Iterator for BlobSource {
    type Item = StageResult<Bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
