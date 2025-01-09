//! Contains a concrete implementation of the [KeyValueStore] trait that stores data on disk.

use super::KeyValueStore;
use crate::cli::HostCli;
use alloy_primitives::{keccak256, B256};
use anyhow::Result;
use kona_preimage::PreimageKey;
use kona_proof_interop::boot::{
    AGREED_L2_PRE_STATE_KEY, CLAIMED_L2_POST_STATE_KEY, L1_HEAD_KEY, L2_CHAIN_ID_KEY,
    L2_CLAIM_TIMESTAMP_KEY, L2_ROLLUP_CONFIG_KEY,
};

/// The default chain ID to use if none is provided.
const DEFAULT_CHAIN_ID: u64 = 0xbeefbabe;

/// A simple, synchronous key-value store that returns data from a [HostCli] config.
#[derive(Debug)]
pub struct LocalKeyValueStore {
    cfg: HostCli,
}

impl LocalKeyValueStore {
    /// Create a new [LocalKeyValueStore] with the given [HostCli] config.
    pub const fn new(cfg: HostCli) -> Self {
        Self { cfg }
    }
}

impl KeyValueStore for LocalKeyValueStore {
    fn get(&self, key: B256) -> Option<Vec<u8>> {
        let preimage_key = PreimageKey::try_from(*key).ok()?;
        match preimage_key.key_value() {
            L1_HEAD_KEY => Some(self.cfg.l1_head.to_vec()),
            AGREED_L2_PRE_STATE_KEY => {
                let hash = keccak256(self.cfg.agreed_pre_state.as_ref());
                dbg!(&self.cfg.agreed_pre_state);
                dbg!(hash);
                Some(hash.to_vec())
            },
            CLAIMED_L2_POST_STATE_KEY => Some(self.cfg.claimed_l2_output_root.to_vec()),
            L2_CLAIM_TIMESTAMP_KEY => {
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
