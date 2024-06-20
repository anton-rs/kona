//! Contains the concrete implementation of the [L2ChainProvider] trait for the client program.

use crate::{BootInfo, CachingOracle, HintType, HINT_WRITER};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_consensus::Header;
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_derive::traits::L2ChainProvider;
use kona_mpt::{OrderedListWalker, TrieDBFetcher};
use kona_preimage::{HintWriterClient, PreimageKey, PreimageKeyType, PreimageOracleClient};
use kona_primitives::{
    L2BlockInfo, L2ExecutionPayloadEnvelope, OpBlock, RollupConfig, SystemConfig,
};
use op_alloy_consensus::{Decodable2718, OpTxEnvelope};

/// The oracle-backed L2 chain provider for the client program.
#[derive(Debug, Clone)]
pub struct OracleL2ChainProvider {
    /// The boot information
    boot_info: Arc<BootInfo>,
    /// The preimage oracle client.
    oracle: Arc<CachingOracle>,
}

impl OracleL2ChainProvider {
    /// Creates a new [OracleL2ChainProvider] with the given boot information and oracle client.
    pub fn new(boot_info: Arc<BootInfo>, oracle: Arc<CachingOracle>) -> Self {
        Self { boot_info, oracle }
    }
}

impl OracleL2ChainProvider {
    /// Returns a [Header] corresponding to the given L2 block number, by walking back from the
    /// L2 safe head.
    async fn header_by_number(&mut self, block_number: u64) -> Result<Header> {
        // Fetch the starting L2 output preimage.
        HINT_WRITER
            .write(
                &HintType::StartingL2Output.encode_with(&[self.boot_info.l2_output_root.as_ref()]),
            )
            .await?;
        let output_preimage = self
            .oracle
            .get(PreimageKey::new(*self.boot_info.l2_output_root, PreimageKeyType::Keccak256))
            .await?;

        // Fetch the starting block header.
        let block_hash = output_preimage[96..128]
            .try_into()
            .map_err(|e| anyhow!("Failed to extract block hash from output preimage: {e}"))?;
        let mut header = self.header_by_hash(block_hash)?;

        // Check if the block number is in range. If not, we can fail early.
        if block_number > header.number {
            anyhow::bail!("Block number past L1 head.");
        }

        // Walk back the block headers to the desired block number.
        while header.number > block_number {
            header = self.header_by_hash(header.parent_hash)?;
        }

        Ok(header)
    }
}

#[async_trait]
impl L2ChainProvider for OracleL2ChainProvider {
    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo> {
        // Get the payload at the given block number.
        let payload = self.payload_by_number(number).await?;

        // Construct the system config from the payload.
        payload.to_l2_block_ref(&self.boot_info.rollup_config)
    }

    async fn payload_by_number(&mut self, number: u64) -> Result<L2ExecutionPayloadEnvelope> {
        // Fetch the header for the given block number.
        let header @ Header { transactions_root, timestamp, .. } =
            self.header_by_number(number).await?;
        let header_hash = header.hash_slow();

        // Fetch the transactions in the block.
        HINT_WRITER.write(&HintType::L2Transactions.encode_with(&[header_hash.as_ref()])).await?;
        let trie_walker = OrderedListWalker::try_new_hydrated(transactions_root, self)?;

        // Decode the transactions within the transactions trie.
        let transactions = trie_walker
            .into_iter()
            .map(|(_, rlp)| {
                OpTxEnvelope::decode_2718(&mut rlp.as_ref())
                    .map_err(|e| anyhow!("Failed to decode TxEnvelope RLP: {e}"))
            })
            .collect::<Result<Vec<_>>>()?;

        let optimism_block = OpBlock {
            header,
            body: transactions,
            withdrawals: self.boot_info.rollup_config.is_canyon_active(timestamp).then(Vec::new),
            ..Default::default()
        };
        Ok(optimism_block.into())
    }

    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> Result<SystemConfig> {
        // Get the payload at the given block number.
        let payload = self.payload_by_number(number).await?;

        // Construct the system config from the payload.
        payload.to_system_config(rollup_config.as_ref())
    }
}

impl TrieDBFetcher for OracleL2ChainProvider {
    fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
        // On L2, trie node preimages are stored as keccak preimage types in the oracle. We assume
        // that a hint for these preimages has already been sent, prior to this call.
        kona_common::block_on(async move {
            self.oracle
                .get(PreimageKey::new(*key, PreimageKeyType::Keccak256))
                .await
                .map(Into::into)
        })
    }

    fn bytecode_by_hash(&self, hash: B256) -> Result<Bytes> {
        // Fetch the bytecode preimage from the caching oracle.
        kona_common::block_on(async move {
            HINT_WRITER.write(&HintType::L2Code.encode_with(&[hash.as_ref()])).await?;

            self.oracle
                .get(PreimageKey::new(*hash, PreimageKeyType::Keccak256))
                .await
                .map(Into::into)
        })
    }

    fn header_by_hash(&self, hash: B256) -> Result<Header> {
        // Fetch the header from the caching oracle.
        kona_common::block_on(async move {
            HINT_WRITER.write(&HintType::L2BlockHeader.encode_with(&[hash.as_ref()])).await?;

            let header_bytes =
                self.oracle.get(PreimageKey::new(*hash, PreimageKeyType::Keccak256)).await?;
            Header::decode(&mut header_bytes.as_slice())
                .map_err(|e| anyhow!("Failed to RLP decode Header: {e}"))
        })
    }
}
