//! Data Sources Test Utilities

use crate::{
    traits::{ChainProvider, L2ChainProvider},
    types::{BlockInfo, L2BlockInfo, L2ExecutionPayloadEnvelope},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_consensus::{Header, Receipt, TxEnvelope};
use alloy_primitives::B256;
use anyhow::Result;
use async_trait::async_trait;

/// A mock block fetcher.
#[derive(Debug, Default)]
pub struct MockBlockFetcher {
    /// Blocks
    pub blocks: Vec<L2BlockInfo>,
    /// Payloads
    pub payloads: Vec<L2ExecutionPayloadEnvelope>,
}

impl MockBlockFetcher {
    /// Creates a new [MockBlockFetcher] with the given origin and batches.
    pub fn new(blocks: Vec<L2BlockInfo>, payloads: Vec<L2ExecutionPayloadEnvelope>) -> Self {
        Self { blocks, payloads }
    }
}

#[async_trait]
impl L2ChainProvider for MockBlockFetcher {
    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo> {
        self.blocks
            .iter()
            .find(|b| b.block_info.number == number)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Block not found"))
    }

    async fn payload_by_number(&mut self, number: u64) -> Result<L2ExecutionPayloadEnvelope> {
        self.payloads
            .iter()
            .find(|p| p.execution_payload.block_number == number)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Payload not found"))
    }
}

/// A mock chain provider for testing.
#[derive(Debug, Clone, Default)]
pub struct TestChainProvider {
    /// Maps block numbers to block information using a tuple list.
    pub blocks: Vec<(u64, BlockInfo)>,
    /// Maps block hashes to header information using a tuple list.
    pub headers: Vec<(B256, Header)>,
    /// Maps block hashes to receipts using a tuple list.
    pub receipts: Vec<(B256, Vec<Receipt>)>,
}

impl TestChainProvider {
    /// Insert a block into the mock chain provider.
    pub fn insert_block(&mut self, number: u64, block: BlockInfo) {
        self.blocks.push((number, block));
    }

    /// Insert receipts into the mock chain provider.
    pub fn insert_receipts(&mut self, hash: B256, receipts: Vec<Receipt>) {
        self.receipts.push((hash, receipts));
    }

    /// Insert a header into the mock chain provider.
    pub fn insert_header(&mut self, hash: B256, header: Header) {
        self.headers.push((hash, header));
    }

    /// Clears headers from the mock chain provider.
    pub fn clear_headers(&mut self) {
        self.headers.clear();
    }

    /// Clears blocks from the mock chain provider.
    pub fn clear_blocks(&mut self) {
        self.blocks.clear();
    }

    /// Clears receipts from the mock chain provider.
    pub fn clear_receipts(&mut self) {
        self.receipts.clear();
    }

    /// Clears all blocks and receipts from the mock chain provider.
    pub fn clear(&mut self) {
        self.clear_blocks();
        self.clear_receipts();
        self.clear_headers();
    }
}

#[async_trait]
impl ChainProvider for TestChainProvider {
    async fn header_by_hash(&mut self, hash: B256) -> Result<Header> {
        if let Some((_, header)) = self.headers.iter().find(|(_, b)| b.hash_slow() == hash) {
            Ok(header.clone())
        } else {
            Err(anyhow::anyhow!("Header not found"))
        }
    }

    async fn block_info_by_number(&mut self, _number: u64) -> Result<BlockInfo> {
        if let Some((_, block)) = self.blocks.iter().find(|(n, _)| *n == _number) {
            Ok(*block)
        } else {
            Err(anyhow::anyhow!("Block not found"))
        }
    }

    async fn receipts_by_hash(&mut self, _hash: B256) -> Result<Vec<Receipt>> {
        if let Some((_, receipts)) = self.receipts.iter().find(|(h, _)| *h == _hash) {
            Ok(receipts.clone())
        } else {
            Err(anyhow::anyhow!("Receipts not found"))
        }
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        let block = self
            .blocks
            .iter()
            .find(|(_, b)| b.hash == hash)
            .map(|(_, b)| *b)
            .ok_or_else(|| anyhow::anyhow!("Block not found"))?;
        Ok((block, Vec::new()))
    }
}
