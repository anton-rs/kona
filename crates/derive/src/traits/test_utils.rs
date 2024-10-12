//! Test Utilities for derive traits

use crate::{
    errors::{BlobProviderError, PipelineError, PipelineResult},
    traits::{AsyncIterator, BlobProvider, DataAvailabilityProvider},
};
use alloc::{boxed::Box, vec, vec::Vec};
use alloy_eips::{eip1898::NumHash, eip4844::Blob};
use alloy_primitives::{map::HashMap, Address, Bytes, B256};
use anyhow::Result;
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_protocol::BlockInfo;

/// Mock data iterator
#[derive(Debug, Default, PartialEq)]
pub struct TestIter {
    /// Holds open data calls with args for assertions.
    pub(crate) open_data_calls: Vec<(BlockInfo, Address)>,
    /// A queue of results to return as the next iterated data.
    pub(crate) results: Vec<PipelineResult<Bytes>>,
}

#[async_trait]
impl AsyncIterator for TestIter {
    type Item = Bytes;

    async fn next(&mut self) -> PipelineResult<Self::Item> {
        self.results.pop().unwrap_or(Err(PipelineError::Eof.temp()))
    }
}

/// Mock data availability provider
#[derive(Debug, Default)]
pub struct TestDAP {
    /// The batch inbox address.
    pub batch_inbox_address: Address,
    /// Specifies the stage results the test iter returns as data.
    pub(crate) results: Vec<PipelineResult<Bytes>>,
}

#[async_trait]
impl DataAvailabilityProvider for TestDAP {
    type Item = Bytes;
    type DataIter = TestIter;

    async fn open_data(&self, block_ref: &BlockInfo) -> PipelineResult<Self::DataIter> {
        // Construct a new vec of results to return.
        let results = self
            .results
            .iter()
            .map(|i| i.as_ref().map_or_else(|_| Err(PipelineError::Eof.temp()), |r| Ok(r.clone())))
            .collect::<Vec<PipelineResult<Bytes>>>();
        Ok(TestIter { open_data_calls: vec![(*block_ref, self.batch_inbox_address)], results })
    }
}

/// A mock blob provider for testing.
#[derive(Debug, Clone, Default)]
pub struct TestBlobProvider {
    /// Maps block hashes to blob data.
    pub blobs: HashMap<B256, Blob>,
    /// whether the blob provider should return an error.
    pub should_error: bool,
}

impl TestBlobProvider {
    /// Insert a blob into the mock blob provider.
    pub fn insert_blob(&mut self, hash: B256, blob: Blob) {
        self.blobs.insert(hash, blob);
    }

    /// Clears blobs from the mock blob provider.
    pub fn clear(&mut self) {
        self.blobs.clear();
    }
}

#[async_trait]
impl BlobProvider for TestBlobProvider {
    type Error = BlobProviderError;

    async fn get_blobs(
        &mut self,
        _block_ref: &BlockInfo,
        blob_hashes: &[NumHash],
    ) -> Result<Vec<Box<Blob>>, Self::Error> {
        if self.should_error {
            return Err(BlobProviderError::SlotDerivation);
        }
        let mut blobs = Vec::new();
        for blob_hash in blob_hashes {
            if let Some(data) = self.blobs.get(&blob_hash.hash) {
                blobs.push(Box::new(*data));
            }
        }
        Ok(blobs)
    }
}
