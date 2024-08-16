//! Fixture providers using data from the derivation test fixture.

use alloy_consensus::{Header, Receipt, TxEnvelope};
use alloy_eips::eip2718::Decodable2718;
use alloy_primitives::B256;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_derive::{
    traits::{ChainProvider, L2ChainProvider},
    types::{
        BlockInfo, L2BlockInfo, L2ExecutionPayload, L2ExecutionPayloadEnvelope, RollupConfig,
        SystemConfig,
    },
};
use op_test_vectors::derivation::DerivationFixture;
use std::sync::Arc;

/// An L1Provider using the fixture data from the derivation test fixture.
#[derive(Debug, Clone)]
pub struct FixtureL1Provider {
    inner: DerivationFixture,
}

impl From<DerivationFixture> for FixtureL1Provider {
    fn from(inner: DerivationFixture) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl ChainProvider for FixtureL1Provider {
    async fn header_by_hash(&mut self, hash: B256) -> Result<Header> {
        let Some(l1_block) = self.inner.l1_blocks.iter().find(|b| b.header.hash_slow() == hash)
        else {
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
        let Some(l1_block) = self.inner.l1_blocks.iter().find(|b| b.header.hash_slow() == hash)
        else {
            return Err(anyhow!("Block not found"));
        };
        Ok(l1_block.receipts.clone())
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        let Some(l1_block) = self.inner.l1_blocks.iter().find(|b| b.header.hash_slow() == hash)
        else {
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

impl From<DerivationFixture> for FixtureL2Provider {
    fn from(inner: DerivationFixture) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl L2ChainProvider for FixtureL2Provider {
    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo> {
        self.inner.l2_block_infos.get(&number).cloned().ok_or_else(|| anyhow!("Block not found"))
    }

    async fn payload_by_number(&mut self, number: u64) -> Result<L2ExecutionPayloadEnvelope> {
        let l2_block = self.l2_block_info_by_number(number).await?;
        let mut payload = self.inner.l2_payloads.get(&number);
        if payload.is_none() {
            payload = self.inner.ref_payloads.get(&number);
        }
        let Some(payload) = payload else {
            return Err(anyhow!("Payload not found"));
        };
        Ok(L2ExecutionPayloadEnvelope {
            parent_beacon_block_root: payload.parent_beacon_block_root,
            execution_payload: L2ExecutionPayload {
                parent_hash: l2_block.block_info.parent_hash,
                fee_recipient: payload.fee_recipient,
                prev_randao: payload.prev_randao,
                block_number: l2_block.block_info.number,
                gas_limit: payload.gas_limit.unwrap_or_default() as u128,
                timestamp: l2_block.block_info.timestamp,
                block_hash: l2_block.block_info.hash,
                transactions: payload.transactions.clone().into_iter().map(|tx| tx.0).collect(),
                withdrawals: payload.withdrawals.clone(),
                // These fields aren't used in derivation for span batch checking anyways.
                state_root: Default::default(),
                receipts_root: Default::default(),
                logs_bloom: Default::default(),
                gas_used: Default::default(),
                extra_data: Default::default(),
                base_fee_per_gas: Default::default(),
                deserialized_transactions: Default::default(),
                blob_gas_used: Default::default(),
                excess_blob_gas: Default::default(),
            },
        })
    }

    async fn system_config_by_number(
        &mut self,
        number: u64,
        _: Arc<RollupConfig>,
    ) -> Result<SystemConfig> {
        self.inner
            .l2_system_configs
            .get(&number)
            .cloned()
            .ok_or_else(|| anyhow!("System config not found"))
    }
}
