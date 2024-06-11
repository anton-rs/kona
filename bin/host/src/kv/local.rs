//! Contains a concrete implementation of the [KeyValueStore] trait that stores data on disk.

use super::KeyValueStore;
use crate::cli::HostCli;
use alloy_primitives::B256;
use kona_client::{
    L1_HEAD_KEY, L2_CHAIN_CONFIG_KEY, L2_CHAIN_ID_KEY, L2_CLAIM_BLOCK_NUMBER_KEY, L2_CLAIM_KEY,
    L2_OUTPUT_ROOT_KEY, L2_ROLLUP_CONFIG_KEY,
};
use kona_preimage::PreimageKey;

/// A simple, synchronous key-value store that returns data from a [HostCli] config.
pub struct LocalKeyValueStore {
    cfg: HostCli,
}

impl LocalKeyValueStore {
    /// Create a new [LocalKeyValueStore] with the given [HostCli] config.
    pub fn new(cfg: HostCli) -> Self {
        Self { cfg }
    }
}

impl KeyValueStore for LocalKeyValueStore {
    fn get(&self, key: B256) -> Option<Vec<u8>> {
        let preimage_key = PreimageKey::try_from(*key).ok()?;
        match preimage_key.key_value() {
            L1_HEAD_KEY => Some(self.cfg.l1_head.to_vec()),
            L2_OUTPUT_ROOT_KEY => Some(self.cfg.l2_output_root.to_vec()),
            L2_CLAIM_KEY => Some(self.cfg.l2_claim.to_vec()),
            L2_CLAIM_BLOCK_NUMBER_KEY => Some(self.cfg.l2_block_number.to_be_bytes().to_vec()),
            L2_CHAIN_ID_KEY => todo!(),
            L2_CHAIN_CONFIG_KEY => todo!(),
            L2_ROLLUP_CONFIG_KEY => todo!(),
            _ => None,
        }
    }

    fn set(&mut self, _: B256, _: Vec<u8>) {
        unreachable!("LocalKeyValueStore is read-only")
    }
}
