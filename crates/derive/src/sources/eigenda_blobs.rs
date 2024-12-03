//! Blob Data Source

use crate::{
    errors::{BlobProviderError, PipelineError},
    sources::BlobData,
    traits::{BlobProvider, ChainProvider, DataAvailabilityProvider},
    types::PipelineResult,
};
use alloc::{boxed::Box, string::ToString, vec::Vec};
use alloy_consensus::{Transaction, TxEip4844Variant, TxEnvelope, TxType};
use alloy_eips::eip4844::IndexedBlobHash;
use alloy_primitives::{Address, Bytes};
use async_trait::async_trait;
use op_alloy_protocol::BlockInfo;

/// A data iterator that reads from a blob.
#[derive(Debug, Clone)]
pub struct EigenDABlobSource<B>
where
    B: AltDAProvider + Send,
{
    /// Fetches blobs.
    pub alta_fetcher: B,
    /// EigenDA blobs.
    pub data: Vec<Vec<Bytes>>,
    /// Whether the source is open.
    pub open: bool,
}

impl<F, B> EigenDABlobSource<F, B>
where
    B: BlobProvider + Send,
{
    /// Creates a new blob source.
    pub const fn new(
        altda_fetcher: B,
    ) -> Self {
        Self {
            altda_fetcher,
        }
    }

    fn extract_blob_data(&self, txs: Vec<TxEnvelope>) -> (Vec<BlobData>, Vec<IndexedBlobHash>) {
        
    }

    /// Loads blob data into the source if it is not open.
    async fn load_blobs(&mut self, altDACommitment: &AltDACommitment) -> Result<(), BlobProviderError> {
        
    }

    fn next_data(&mut self) -> Result<EigenDABlobData, PipelineResult<Bytes>> {
        if self.open{
            
        }

        if self.data.is_empty() {
            return Err(Err(PipelineError::Eof.temp()));
        }
        Ok(self.data.remove(0))
    }
}

impl<AP: AltDAProvider + Send> DataAvailabilityProvider for EigenDABlobSource<AP> {
    type Item = Bytes;

    async fn next(&mut self, altDACommitment: &AltDACommitment) -> PipelineResult<Self::Item> {
        self.load_blobs(altDACommitment).await?;

        let next_data = match self.next_data() {
            Ok(d) => d,
            Err(e) => return e,
        };
        if let Some(c) = next_data {
            return Ok(c);
        }

        // Decode the blob data to raw bytes.
        // Otherwise, ignore blob and recurse next.
        match next_data.decode() {
            Ok(d) => Ok(d),
            Err(_) => {
                warn!(target: "blob-source", "Failed to decode blob data, skipping");
                self.next(block_ref).await
            }
        }
    }

    fn clear(&mut self) {
        self.data.clear();
        self.open = false;
    }
}