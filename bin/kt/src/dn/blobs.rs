//! Blob Fixture Provider.

use super::LocalDerivationFixture;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_derive::{errors::BlobProviderError, traits::BlobProvider};
use kona_primitives::{Blob, BlockInfo, IndexedBlobHash};

/// A blob fixture provider.
#[derive(Debug, Clone)]
pub(crate) struct BlobFixtureProvider {
    inner: LocalDerivationFixture,
}

impl From<LocalDerivationFixture> for BlobFixtureProvider {
    fn from(inner: LocalDerivationFixture) -> Self {
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
