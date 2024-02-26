//! Blob Data Source

use crate::traits::DataIter;
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

impl<T: Into<Bytes>> DataIter<T> for BlobSource {
    fn next(&mut self) -> StageResult<T> {
        unimplemented!()
    }
}
