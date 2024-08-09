//! Blob Fixture Provider.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_derive::{
    traits::BlobProvider,
    types::{Blob, BlobProviderError, BlockInfo, IndexedBlobHash},
};
use op_test_vectors::derivation::DerivationFixture;

/// A blob fixture provider.
#[derive(Debug, Clone)]
pub struct BlobFixtureProvider {
    inner: DerivationFixture,
}

impl From<DerivationFixture> for BlobFixtureProvider {
    fn from(inner: DerivationFixture) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl BlobProvider for BlobFixtureProvider {
    async fn get_blobs(
        &mut self,
        block_ref: &BlockInfo,
        _blob_hashes: &[IndexedBlobHash],
    ) -> Result<Vec<Blob>, BlobProviderError> {
        let Some(l1_block) =
            self.inner.l1_blocks.iter().find(|b| b.header.number == block_ref.number)
        else {
            return Err(BlobProviderError::Custom(anyhow!("Blob not found")));
        };
        // TODO: do we need to check blob hashes?
        Ok(l1_block.blobs.clone().into_iter().map(|b| *b).collect())
    }
}
