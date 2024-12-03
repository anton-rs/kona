//! Blob Data Source

use crate::{
    errors::{BlobProviderError, PipelineError},
    sources::EigenDABlobData,
    traits::{BlobProvider, ChainProvider, DataAvailabilityProvider, EigenDABlobProvider},
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
    B: EigenDABlobProvider + Send,
{
    /// Fetches blobs.
    pub altda_fetcher: B,
    /// EigenDA blobs.
    pub data: Vec<EigenDABlobData>,
    /// Whether the source is open.
    pub open: bool,
}

impl<B> EigenDABlobSource<B>
where
    B: EigenDABlobProvider + Send,
{
    /// Creates a new blob source.
    pub const fn new(
        altda_fetcher: B,
    ) -> Self {
        Self {
            altda_fetcher,
            data: Vec::new(),
            open: false,
        }
    }

    fn extract_blob_data(&self, txs: Vec<TxEnvelope>) -> (Vec<EigenDABlobData>, Vec<IndexedBlobHash>) {
        todo!()
    }

    /// Loads blob data into the source if it is not open.
    async fn load_blobs(&mut self, altDACommitment: &Bytes) -> Result<(), BlobProviderError> {
        todo!()
    }

    fn next_data(&mut self) -> Result<EigenDABlobData, PipelineResult<Bytes>> {
        if self.open{
            return Err(Err(PipelineError::Eof.temp()));
        }

        if self.data.is_empty() {
            return Err(Err(PipelineError::Eof.temp()));
        }
        Ok(self.data.remove(0))
    }

    pub async fn next(&mut self, altDACommitment: &Bytes) -> PipelineResult<Bytes> {
        self.load_blobs(altDACommitment).await?;

        let next_data = match self.next_data() {
            Ok(d) => d,
            Err(e) => return e,
        };

        // Decode the blob data to raw bytes.
        // Otherwise, ignore blob and recurse next.
        match next_data.decode() {
            Ok(d) => Ok(d),
            Err(_) => {
                warn!(target: "blob-source", "Failed to decode blob data, skipping");
                panic!()
                // todo need to add recursion
                // self.next(altDACommitment).await
            }
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.open = false;
    }
}