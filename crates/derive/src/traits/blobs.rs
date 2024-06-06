//! Blob Provider

use crate::types::{Blob, BlobProviderError, BlockInfo, IndexedBlobHash};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_primitives::B256;
use anyhow::Result;
use async_trait::async_trait;
use hashbrown::HashMap;
use spin::Mutex;

/// The BlobProvider trait specifies the functionality of a data source that can provide blobs.
#[async_trait]
pub trait BlobProvider {
    /// Fetches blobs for a given block ref and the blob hashes.
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Blob>, BlobProviderError>;
}

/// A blob provider that hold blobs in memory.
#[derive(Default, Debug, Clone)]
pub struct InMemoryBlobStore {
    /// Maps block hashes to blobs and their indexed hashes.
    blocks_to_blob: HashMap<B256, Vec<(IndexedBlobHash, Blob)>>,
}

impl InMemoryBlobStore {
    /// Creates a new [InMemoryBlobStore].
    pub fn new() -> Self {
        Self { blocks_to_blob: HashMap::new() }
    }

    /// Inserts multiple blobs into the provider.
    pub fn insert_blobs(&mut self, block_hash: B256, blobs: Vec<(IndexedBlobHash, Blob)>) {
        self.blocks_to_blob.entry(block_hash).or_default().extend(blobs);
    }
}

/// [BlobProvider] implementation that wraps an in memory blob store.
#[derive(Debug, Clone)]
pub struct WrappedBlobProvider(Arc<Mutex<InMemoryBlobStore>>);

impl WrappedBlobProvider {
    /// Creates a new [WrappedBlobProvider].
    pub fn new(inner: Arc<Mutex<InMemoryBlobStore>>) -> Self {
        Self(inner)
    }
}

#[async_trait]
impl BlobProvider for WrappedBlobProvider {
    /// Fetches blobs for a given block ref and the blob hashes.
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Blob>, BlobProviderError> {
        let err = |block_ref: &BlockInfo| {
            BlobProviderError::Custom(anyhow::anyhow!(
                "Blob not found for block ref: {:?}",
                block_ref
            ))
        };
        let locked = self.0.lock();
        let blobs_for_block =
            locked.blocks_to_blob.get(&block_ref.hash).ok_or_else(|| err(block_ref))?;
        let mut blobs = Vec::new();
        for (hash, blob) in blobs_for_block {
            if blob_hashes.contains(hash) {
                blobs.push(*blob);
            }
        }
        Ok(blobs)
    }
}
