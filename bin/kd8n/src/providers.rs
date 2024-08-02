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
        let Some(l1_block) = self.inner.l1_blocks.find(|b| b.hash == hash) else {
            return Err(anyhow!("Block not found"));
        };
        Ok(Header {
            hash,
            number: l1_block.number,
            timestamp: l1_block.timestamp,
            ..Default::default()
        })
    }

    async fn block_info_by_number(&mut self, number: u64) -> Result<BlockInfo> {
        unimplemented!()
    }

    async fn receipts_by_hash(&mut self, hash: B256) -> Result<Vec<Receipt>> {
        unimplemented!()
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        unimplemented!()
    }
}
