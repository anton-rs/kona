//! Implements a mock [L2SystemConfigFetcher] for testing.

use crate::traits::L2ChainProvider;
use alloc::{boxed::Box, sync::Arc};
use anyhow::Result;
use async_trait::async_trait;
use hashbrown::HashMap;
use kona_primitives::L2ExecutionPayloadEnvelope;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::L2BlockInfo;

/// A mock implementation of the [`SystemConfigL2Fetcher`] for testing.
#[derive(Debug, Default)]
pub struct MockSystemConfigL2Fetcher {
    /// A map from [u64] block number to a [SystemConfig].
    pub system_configs: HashMap<u64, SystemConfig>,
}

impl MockSystemConfigL2Fetcher {
    /// Inserts a new system config into the mock fetcher with the given block number.
    pub fn insert(&mut self, number: u64, config: SystemConfig) {
        self.system_configs.insert(number, config);
    }

    /// Clears all system configs from the mock fetcher.
    pub fn clear(&mut self) {
        self.system_configs.clear();
    }
}

#[async_trait]
impl L2ChainProvider for MockSystemConfigL2Fetcher {
    async fn system_config_by_number(
        &mut self,
        number: u64,
        _: Arc<RollupConfig>,
    ) -> Result<SystemConfig> {
        self.system_configs
            .get(&number)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("system config not found: {number}"))
    }

    async fn l2_block_info_by_number(&mut self, _: u64) -> Result<L2BlockInfo> {
        unimplemented!()
    }

    async fn payload_by_number(&mut self, _: u64) -> Result<L2ExecutionPayloadEnvelope> {
        unimplemented!()
    }
}
