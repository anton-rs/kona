//! Contains the concrete implementation of the [ChainProvider] trait for the client program.

use crate::{BootInfo, HintType};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_consensus::{Header, Receipt, ReceiptEnvelope, TxEnvelope};
use alloy_eips::eip2718::Decodable2718;
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_derive::traits::ChainProvider;
use kona_mpt::{OrderedListWalker, TrieProvider};
use kona_preimage::{CommsClient, PreimageKey, PreimageKeyType};
use op_alloy_protocol::BlockInfo;

/// The oracle-backed L1 chain provider for the client program.
#[derive(Debug, Clone)]
pub struct OracleL1ChainProvider<T: CommsClient> {
    /// The boot information
    boot_info: Arc<BootInfo>,
    /// The preimage oracle client.
    pub oracle: Arc<T>,
}

impl<T: CommsClient> OracleL1ChainProvider<T> {
    /// Creates a new [OracleL1ChainProvider] with the given boot information and oracle client.
    pub fn new(boot_info: Arc<BootInfo>, oracle: Arc<T>) -> Self {
        Self { boot_info, oracle }
    }
}

#[async_trait]
impl<T: CommsClient + Sync + Send> ChainProvider for OracleL1ChainProvider<T> {
    type Error = anyhow::Error;

    async fn header_by_hash(&mut self, hash: B256) -> Result<Header> {
        // Send a hint for the block header.
        self.oracle.write(&HintType::L1BlockHeader.encode_with(&[hash.as_ref()])).await?;

        // Fetch the header RLP from the oracle.
        let header_rlp =
            self.oracle.get(PreimageKey::new(*hash, PreimageKeyType::Keccak256)).await?;

        // Decode the header RLP into a Header.
        Header::decode(&mut header_rlp.as_slice())
            .map_err(|e| anyhow!("Failed to decode header RLP: {e}"))
    }

    async fn block_info_by_number(&mut self, block_number: u64) -> Result<BlockInfo> {
        // Fetch the starting block header.
        let mut header = self.header_by_hash(self.boot_info.l1_head).await?;

        // Check if the block number is in range. If not, we can fail early.
        if block_number > header.number {
            anyhow::bail!("Block number past L1 head.");
        }

        // Walk back the block headers to the desired block number.
        while header.number > block_number {
            header = self.header_by_hash(header.parent_hash).await?;
        }

        Ok(BlockInfo {
            hash: header.hash_slow(),
            number: header.number,
            parent_hash: header.parent_hash,
            timestamp: header.timestamp,
        })
    }

    async fn receipts_by_hash(&mut self, hash: B256) -> Result<Vec<Receipt>> {
        // Fetch the block header to find the receipts root.
        let header = self.header_by_hash(hash).await?;

        // Send a hint for the block's receipts, and walk through the receipts trie in the header to
        // verify them.
        self.oracle.write(&HintType::L1Receipts.encode_with(&[hash.as_ref()])).await?;
        let trie_walker = OrderedListWalker::try_new_hydrated(header.receipts_root, self)?;

        // Decode the receipts within the transactions trie.
        let receipts = trie_walker
            .into_iter()
            .map(|(_, rlp)| {
                let envelope = ReceiptEnvelope::decode_2718(&mut rlp.as_ref())
                    .map_err(|e| anyhow!("Failed to decode ReceiptEnvelope RLP: {e}"))?;
                Ok(envelope.as_receipt().expect("Infalliable").clone())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(receipts)
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        // Fetch the block header to construct the block info.
        let header = self.header_by_hash(hash).await?;
        let block_info = BlockInfo {
            hash,
            number: header.number,
            parent_hash: header.parent_hash,
            timestamp: header.timestamp,
        };

        // Send a hint for the block's transactions, and walk through the transactions trie in the
        // header to verify them.
        self.oracle.write(&HintType::L1Transactions.encode_with(&[hash.as_ref()])).await?;
        let trie_walker = OrderedListWalker::try_new_hydrated(header.transactions_root, self)?;

        // Decode the transactions within the transactions trie.
        let transactions = trie_walker
            .into_iter()
            .map(|(_, rlp)| {
                TxEnvelope::decode_2718(&mut rlp.as_ref())
                    .map_err(|e| anyhow!("Failed to decode TxEnvelope RLP: {e}"))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((block_info, transactions))
    }
}

impl<T: CommsClient> TrieProvider for OracleL1ChainProvider<T> {
    type Error = anyhow::Error;

    fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
        // On L1, trie node preimages are stored as keccak preimage types in the oracle. We assume
        // that a hint for these preimages has already been sent, prior to this call.
        kona_common::block_on(async move {
            self.oracle
                .get(PreimageKey::new(*key, PreimageKeyType::Keccak256))
                .await
                .map(Into::into)
                .map_err(Into::into)
        })
    }

    fn bytecode_by_hash(&self, _: B256) -> Result<Bytes> {
        unimplemented!("TrieProvider::bytecode_by_hash unimplemented for OracleL1ChainProvider")
    }

    fn header_by_hash(&self, _: B256) -> Result<Header> {
        unimplemented!("TrieProvider::header_by_hash unimplemented for OracleL1ChainProvider")
    }
}
