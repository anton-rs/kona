//! Fixture providers using data from the derivation test fixture.

use anyhow::Result;
use alloy_eips::eip2718::Decodable2718;
use alloy_consensus::{Header, Receipt, TxEnvelope};
use alloy_primitives::B256;
use anyhow::anyhow;
use async_trait::async_trait;
use kona_derive::{
    traits::{ChainProvider, L2ChainProvider},
    types::{L2BlockInfo, SystemConfig, L2ExecutionPayloadEnvelope, BlockInfo},
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
        let Some(l1_block) = self.inner.l1_blocks.iter().find(|b| b.header.hash_slow() == hash) else {
            return Err(anyhow!("Block not found"));
        };
        Ok(l1_block.header.clone())
    }

    async fn block_info_by_number(&mut self, number: u64) -> Result<BlockInfo> {
        let Some(l1_block) = self.inner.l1_blocks.iter().find(|b| b.header.number == number) else {
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
        let Some(l1_block) = self.inner.l1_blocks.iter().find(|b| b.header.hash_slow() == hash) else {
            return Err(anyhow!("Block not found"));
        };
        Ok(l1_block.receipts.clone())
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        let Some(l1_block) = self.inner.l1_blocks.iter().find(|b| b.header.hash_slow() == hash) else {
            return Err(anyhow!("Block not found"));
        };
        let mut decoded = Vec::with_capacity(l1_block.transactions.len());
        for tx in l1_block.transactions.iter() {
            let decoded_tx = TxEnvelope::decode_2718(&mut &tx[..]).map_err(|e| anyhow!(e))?;
            decoded.push(decoded_tx);
        }
        Ok((
            BlockInfo {
                number: l1_block.header.number,
                hash: l1_block.header.hash_slow(),
                parent_hash: l1_block.header.parent_hash,
                timestamp: l1_block.header.timestamp,
            },
            decoded,
        ))
    }
}

/// An L2Provider using the fixture data from the derivation test fixture.
#[derive(Debug, Clone)]
pub struct FixtureL2Provider {
    inner: DerivationFixture,
}

#[async_trait]
impl L2ChainProvider for FixtureL2Provider {
    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo> {
        let Some(l2_block) = self.inner.l2_blocks.find(|b| b.header.number == number) else {
            return Err(anyhow!("Block not found"));
        };
        Ok(L2BlockInfo {
            number: l2_block.header.number,
            hash: l2_block.header.hash_slow(),
            parent_hash: l2_block.header.parent_hash,
            timestamp: l2_block.header.timestamp,
        })
    }

    async fn payload_by_number(&mut self, number: u64) -> Result<L2ExecutionPayloadEnvelope> {
        let Some(payload) = self.inner.l2_payloads.find(|p| p.header.number == number) else {
            return Err(anyhow!("Payload not found"));
        };
        Ok(L2ExecutionPayloadEnvelope {
            parent_beacon_block_root: 
            execution_payload: payload.clone()
        })
    }

    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> Result<SystemConfig> {
        let Some(l2_block) = self.inner.l2_blocks.find(|b| b.header.number == number) else {
            return Err(anyhow!("Block not found"));
        };
        Ok(l2_block.system_config.clone())
    }
}
