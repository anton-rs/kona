//! Contains a concrete implementation of the [KeyValueStore] trait that stores data on disk.

use super::KeyValueStore;
use crate::cli::HostCli;
use alloy_primitives::B256;
use kona_client::{
    L1_HEAD_KEY, L2_CHAIN_ID_KEY, L2_CLAIM_BLOCK_NUMBER_KEY, L2_CLAIM_KEY, L2_OUTPUT_ROOT_KEY,
    L2_ROLLUP_CONFIG_KEY,
};
use kona_preimage::PreimageKey;
use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};

/// A simple, synchronous key-value store that returns data from a [HostCli] config.
pub struct LocalKeyValueStore {
    cfg: HostCli,
    json_path: Option<PathBuf>,
}

impl LocalKeyValueStore {
    /// Create a new [LocalKeyValueStore] with the given [HostCli] config.
    pub fn new(cfg: HostCli, json_path: Option<PathBuf>) -> Self {
        Self { cfg, json_path }
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
            L2_CHAIN_ID_KEY => Some(self.cfg.l2_chain_id.to_be_bytes().to_vec()),
            L2_ROLLUP_CONFIG_KEY => unimplemented!("L2RollupConfig fetching in local store not implemented. Necessary for chain IDs without a known rollup config."),
            _ => None,
        }
    }

    fn set(&mut self, _: B256, _: Vec<u8>) {
        unreachable!("LocalKeyValueStore is read-only")
    }

    fn export(&self) -> HashMap<B256, Vec<u8>> {
        let mut store = HashMap::new();
        store.insert(
            B256::from(PreimageKey::new_local(L1_HEAD_KEY.to())),
            self.cfg.l1_head.to_vec(),
        );
        store.insert(
            B256::from(PreimageKey::new_local(L2_OUTPUT_ROOT_KEY.to())),
            self.cfg.l2_output_root.to_vec(),
        );
        store.insert(
            B256::from(PreimageKey::new_local(L2_CLAIM_KEY.to())),
            self.cfg.l2_claim.to_vec(),
        );
        store.insert(
            B256::from(PreimageKey::new_local(L2_CLAIM_BLOCK_NUMBER_KEY.to())),
            self.cfg.l2_block_number.to_be_bytes().to_vec(),
        );
        store.insert(
            B256::from(PreimageKey::new_local(L2_CHAIN_ID_KEY.to())),
            self.cfg.l2_chain_id.to_be_bytes().to_vec(),
        );
        store
    }

    fn export_json(&self) {
        if let Some(path) = &self.json_path {
            let store = self.export();
            let json = serde_json::to_string(&store).expect("Failed to serialize to JSON");
            let mut file = File::create(path).expect("Failed to create file");
            file.write_all(json.as_bytes()).expect("Failed to write to file");
        }
    }
}
