//! Implements a mock [L2SystemConfigFetcher] for testing.

use crate::{stages::attributes_queue::SystemConfigL2Fetcher, types::SystemConfig};
use alloy_primitives::B256;
use hashbrown::HashMap;

/// A mock implementation of the [`SystemConfigL2Fetcher`] for testing.
#[derive(Debug, Default)]
pub struct MockSystemConfigL2Fetcher {
    /// A map from [B256] block hash to a [SystemConfig].
    pub system_configs: HashMap<B256, SystemConfig>,
}

impl MockSystemConfigL2Fetcher {
    /// Inserts a new system config into the mock fetcher with the given hash.
    pub fn insert(&mut self, hash: B256, config: SystemConfig) {
        self.system_configs.insert(hash, config);
    }

    /// Clears all system configs from the mock fetcher.
    pub fn clear(&mut self) {
        self.system_configs.clear();
    }
}

impl SystemConfigL2Fetcher for MockSystemConfigL2Fetcher {
    fn system_config_by_l2_hash(&self, hash: B256) -> anyhow::Result<SystemConfig> {
        self.system_configs
            .get(&hash)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("system config not found"))
    }
}
