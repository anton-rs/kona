//! Fixture providers using data from the derivation test fixture.

use alloy_consensus::{Header, Receipt, TxEnvelope};
use alloy_primitives::B256;
use anyhow::anyhow;
use async_trait::async_trait;
use kona_derive::{
    traits::{ChainProvider, L2ChainProvider},
    types::BlockInfo,
};
use op_test_vectors::derivation::DerivationFixture;

/// An L1Provider using the fixture data from the derivation test fixture.
#[derive(Debug, Clone)]
pub struct FixtureL1Provider {
    inner: DerivationFixture,
}

#[async_trait]
impl ChainProvider for FixtureL1Provider {
    async fn header_by_hash(&mut self, hash: B256) -> Result<Header> {
        let Some(l1_block) = self.inner.l1_blocks.find(|b| b.header.hash_slow() == hash) else {
            return Err(anyhow!("Block not found"));
        };
        Ok(l1_block.header)
    }

    async fn block_info_by_number(&mut self, number: u64) -> Result<BlockInfo> {
        let Some(l1_block) = self.inner.l1_blocks.find(|b| b.header.number == number) else {
            return Err(anyhow!("Block not found"));
        };
        Ok(BlockInfo {
            number: l1_block.header.number,
            hash: l1_block.header.hash_slow(),
            parent_hash: l1_block.header.parent_hash,
            timestamp: l1_block.header.timestamp,
        })
    }

    async fn receipts_by_hash(&mut self, hash: B256) -> Result<Vec<Receipt>> {
        let Some(l1_block) = self.inner.l1_blocks.find(|b| b.header.hash_slow() == hash) else {
            return Err(anyhow!("Block not found"));
        };
        Ok(l1_block.receipts.clone())
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        let Some(l1_block) = self.inner.l1_blocks.find(|b| b.header.hash_slow() == hash) else {
            return Err(anyhow!("Block not found"));
        };
        Ok((
            BlockInfo {
                number: l1_block.header.number,
                hash: l1_block.header.hash,
                parent_hash: l1_block.header.parent_hash,
                timestamp: l1_block.header.timestamp,
            },
            l1_block.transactions.clone(),
        ))
    }
}
