//! Contains a concrete implementation of the [KeyValueStore] trait that stores data on disk,
//! using the [SingleChainHostCli] config.

use super::SingleChainHostCli;
use alloy_primitives::B256;
use anyhow::Result;
use kona_preimage_server::KeyValueStore;
use kona_preimage::PreimageKey;
use kona_proof::boot::{
    L1_HEAD_KEY, L2_CHAIN_ID_KEY, L2_CLAIM_BLOCK_NUMBER_KEY, L2_CLAIM_KEY, L2_OUTPUT_ROOT_KEY,
    L2_ROLLUP_CONFIG_KEY,
};

/// The default chain ID to use if none is provided.
const DEFAULT_CHAIN_ID: u64 = 0xbeefbabe;

/// A simple, synchronous key-value store that returns data from a [SingleChainHostCli] config.
#[derive(Debug)]
pub struct LocalKeyValueStore {
    cfg: SingleChainHostCli,
}

impl LocalKeyValueStore {
    /// Create a new [LocalKeyValueStore] with the given [SingleChainHostCli] config.
    pub const fn new(cfg: SingleChainHostCli) -> Self {
        Self { cfg }
    }
}

impl KeyValueStore for LocalKeyValueStore {
    fn get(&self, key: B256) -> Option<Vec<u8>> {
        let preimage_key = PreimageKey::try_from(*key).ok()?;
        match preimage_key.key_value() {
            L1_HEAD_KEY => Some(self.cfg.l1_head.to_vec()),
            L2_OUTPUT_ROOT_KEY => Some(self.cfg.agreed_l2_output_root.to_vec()),
            L2_CLAIM_KEY => Some(self.cfg.claimed_l2_output_root.to_vec()),
            L2_CLAIM_BLOCK_NUMBER_KEY => {
                Some(self.cfg.claimed_l2_block_number.to_be_bytes().to_vec())
            }
            L2_CHAIN_ID_KEY => {
                Some(self.cfg.l2_chain_id.unwrap_or(DEFAULT_CHAIN_ID).to_be_bytes().to_vec())
            }
            L2_ROLLUP_CONFIG_KEY => {
                let rollup_config = self.cfg.read_rollup_config().ok()?;
                let serialized = serde_json::to_vec(&rollup_config).ok()?;
                Some(serialized)
            }
            _ => None,
        }
    }

    fn set(&mut self, _: B256, _: Vec<u8>) -> Result<()> {
        unreachable!("LocalKeyValueStore is read-only")
    }
}
