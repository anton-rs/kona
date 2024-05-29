//! Contains a concrete implementation of the [KeyValueStore] trait that stores data on disk.

use super::KeyValueStore;
use crate::cli::HostCli;
use alloy_primitives::{B256, U256};
use kona_preimage::PreimageKey;

pub(crate) const L1_HEAD_KEY: U256 = U256::from_be_slice(&[1]);
pub(crate) const L2_OUTPUT_ROOT_KEY: U256 = U256::from_be_slice(&[2]);
pub(crate) const L2_CLAIM_KEY: U256 = U256::from_be_slice(&[3]);
pub(crate) const L2_CLAIM_BLOCK_NUMBER_KEY: U256 = U256::from_be_slice(&[4]);
pub(crate) const L2_CHAIN_ID_KEY: U256 = U256::from_be_slice(&[5]);
pub(crate) const L2_CHAIN_CONFIG_KEY: U256 = U256::from_be_slice(&[6]);
pub(crate) const L2_ROLLUP_CONFIG_KEY: U256 = U256::from_be_slice(&[7]);

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
