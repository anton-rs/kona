//! Blob Data Source

use crate::{
    traits::{AsyncIterator, BlobProvider, ChainProvider},
    types::{BlobData, BlockInfo, IndexedBlobHash, StageError, StageResult},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_consensus::{Transaction, TxEip4844Variant, TxEnvelope, TxType};
use alloy_primitives::{Address, Bytes, TxKind};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::warn;

/// A data iterator that reads from a blob.
#[derive(Debug, Clone)]
pub struct BlobSource<F, B>
where
    F: ChainProvider + Send,
    B: BlobProvider + Send,
{
    /// Chain provider.
    chain_provider: F,
    /// Fetches blobs.
    blob_fetcher: B,
    /// The address of the batcher contract.
    batcher_address: Address,
    /// Block Ref
    block_ref: BlockInfo,
    /// The L1 Signer.
    #[allow(dead_code)] // Allow dead code for non-k256 builds.
    signer: Address,
    /// Data.
    data: Vec<BlobData>,
    /// Whether the source is open.
    open: bool,
}

impl<F, B> BlobSource<F, B>
where
    F: ChainProvider + Send,
    B: BlobProvider + Send,
{
    /// Creates a new blob source.
    pub fn new(
        chain_provider: F,
        blob_fetcher: B,
        batcher_address: Address,
        block_ref: BlockInfo,
        signer: Address,
    ) -> Self {
        Self {
            chain_provider,
            blob_fetcher,
            batcher_address,
            block_ref,
            signer,
            data: Vec::new(),
            open: false,
        }
    }

    fn extract_blob_data(&self, txs: Vec<TxEnvelope>) -> (Vec<BlobData>, Vec<IndexedBlobHash>) {
        let mut index = 0;
        let mut data = Vec::new();
        let mut hashes = Vec::new();
        for tx in txs {
            let (tx_kind, calldata, blob_hashes) = match &tx {
                TxEnvelope::Legacy(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
                TxEnvelope::Eip2930(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
                TxEnvelope::Eip1559(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
                TxEnvelope::Eip4844(blob_tx_wrapper) => match blob_tx_wrapper.tx() {
                    TxEip4844Variant::TxEip4844(tx) => {
                        (tx.to(), tx.input.clone(), Some(tx.blob_versioned_hashes.clone()))
                    }
                    TxEip4844Variant::TxEip4844WithSidecar(tx) => {
                        let tx = tx.tx();
                        (tx.to(), tx.input.clone(), Some(tx.blob_versioned_hashes.clone()))
                    }
                },
                _ => continue,
            };
            let TxKind::Call(to) = tx_kind else { continue };

            if to != self.batcher_address {
                index += blob_hashes.map_or(0, |h| h.len());
                continue;
            }
            if tx.recover_signer().unwrap_or_default() != self.signer {
                index += blob_hashes.map_or(0, |h| h.len());
                continue;
            }
            if tx.tx_type() != TxType::Eip4844 {
                let blob_data = BlobData { data: None, calldata: Some(calldata.to_vec().into()) };
                data.push(blob_data);
                continue;
            }
            if !calldata.is_empty() {
                let hash = match &tx {
                    TxEnvelope::Legacy(tx) => Some(tx.hash()),
                    TxEnvelope::Eip2930(tx) => Some(tx.hash()),
                    TxEnvelope::Eip1559(tx) => Some(tx.hash()),
                    TxEnvelope::Eip4844(blob_tx_wrapper) => Some(blob_tx_wrapper.hash()),
                    _ => None,
                };
                warn!(target: "blob-source", "Blob tx has calldata, which will be ignored: {hash:?}");
            }
            let blob_hashes = if let Some(b) = blob_hashes {
                b
            } else {
                continue;
            };
            for blob in blob_hashes {
                let indexed = IndexedBlobHash { hash: blob, index };
                hashes.push(indexed);
                data.push(BlobData::default());
                index += 1;
            }
        }
        (data, hashes)
    }

    /// Loads blob data into the source if it is not open.
    async fn load_blobs(&mut self) -> Result<()> {
        if self.open {
            return Ok(());
        }

        let info =
            self.chain_provider.block_info_and_transactions_by_hash(self.block_ref.hash).await?;

        let (mut data, blob_hashes) = self.extract_blob_data(info.1);

        // If there are no hashes, set the calldata and return.
        if blob_hashes.is_empty() {
            self.open = true;
            self.data = data;
            return Ok(());
        }

        let blobs =
            self.blob_fetcher.get_blobs(&self.block_ref, &blob_hashes).await.map_err(|e| {
                warn!(target: "blob-source", "Failed to fetch blobs: {e}");
                anyhow!("Failed to fetch blobs: {e}")
            })?;

        // Fill the blob pointers.
        let mut blob_index = 0;
        for blob in data.iter_mut() {
            match blob.fill(&blobs, blob_index) {
                Ok(_) => {
                    blob_index += 1;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        self.open = true;
        self.data = data;
        Ok(())
    }

    /// Extracts the next data from the source.
    fn next_data(&mut self) -> Result<BlobData, Option<Result<Bytes, StageError>>> {
        if self.data.is_empty() {
            return Err(Some(Err(StageError::Eof)));
        }

        Ok(self.data.remove(0))
    }
}

#[async_trait]
impl<F, B> AsyncIterator for BlobSource<F, B>
where
    F: ChainProvider + Send,
    B: BlobProvider + Send,
{
    type Item = Bytes;

    async fn next(&mut self) -> Option<StageResult<Self::Item>> {
        if self.load_blobs().await.is_err() {
            return Some(Err(StageError::BlockFetch(self.block_ref.hash)));
        }

        let next_data = match self.next_data() {
            Ok(d) => d,
            Err(e) => return e,
        };
        if next_data.calldata.is_some() {
            return Some(Ok(next_data.calldata.unwrap()));
        }

        // Decode the blob data to raw bytes.
        // Otherwise, ignore blob and recurse next.
        match next_data.decode() {
            Ok(d) => Some(Ok(d)),
            Err(_) => {
                warn!(target: "blob-source", "Failed to decode blob data, skipping");
                self.next().await
            }
        }
    }
}
