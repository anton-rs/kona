//! Providers that use alloy provider types on the backend.

use alloy_consensus::Block;
use alloy_primitives::{Bytes, U64};
use alloy_provider::{Provider, ReqwestProvider};
use alloy_rlp::Decodable;
use alloy_transport::{RpcError, TransportErrorKind, TransportResult};
use async_trait::async_trait;
use kona_derive::{
    errors::{PipelineError, PipelineErrorKind},
    traits::L2ChainProvider,
};
use lru::LruCache;
use maili_genesis::{RollupConfig, SystemConfig};
use maili_protocol::{to_system_config, BatchValidationProvider, L2BlockInfo};
use op_alloy_consensus::{OpBlock, OpTxEnvelope};
use std::{boxed::Box, num::NonZeroUsize, sync::Arc};

const CACHE_SIZE: usize = 16;

/// The [AlloyL2ChainProvider] is a concrete implementation of the [L2ChainProvider] trait,
/// providing data over Ethereum JSON-RPC using an alloy provider as the backend.
///
/// **Note**:
/// This provider fetches data using the `debug_getRawBlock` method. The RPC must support this
/// namespace.
#[derive(Debug, Clone)]
pub struct AlloyL2ChainProvider {
    /// The inner Ethereum JSON-RPC provider.
    inner: ReqwestProvider,
    /// The rollup configuration.
    rollup_config: Arc<RollupConfig>,
    /// `block_by_number` LRU cache.
    block_by_number_cache: LruCache<u64, Block<OpTxEnvelope>>,
    /// `l2_block_info_by_number` LRU cache.
    l2_block_info_by_number_cache: LruCache<u64, L2BlockInfo>,
    /// `system_config_by_l2_hash` LRU cache.
    system_config_by_number_cache: LruCache<u64, SystemConfig>,
}

impl AlloyL2ChainProvider {
    /// Creates a new [AlloyL2ChainProvider] with the given alloy provider and [RollupConfig].
    pub fn new(inner: ReqwestProvider, rollup_config: Arc<RollupConfig>) -> Self {
        Self {
            inner,
            rollup_config,
            block_by_number_cache: LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()),
            l2_block_info_by_number_cache: LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()),
            system_config_by_number_cache: LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()),
        }
    }

    /// Returns the chain ID.
    pub async fn chain_id(&mut self) -> Result<u64, RpcError<TransportErrorKind>> {
        self.inner.get_chain_id().await
    }

    /// Returns the latest L2 block number.
    pub async fn latest_block_number(&mut self) -> Result<u64, RpcError<TransportErrorKind>> {
        self.inner.get_block_number().await
    }

    /// Creates a new [AlloyL2ChainProvider] from the provided [reqwest::Url].
    pub fn new_http(url: reqwest::Url, rollup_config: Arc<RollupConfig>) -> Self {
        let inner = ReqwestProvider::new_http(url);
        Self::new(inner, rollup_config)
    }
}

/// An error for the [AlloyL2ChainProvider].
#[derive(Debug, thiserror::Error)]
pub enum AlloyL2ChainProviderError {
    /// Failed to find a block.
    #[error("Failed to fetch block {0}")]
    BlockNotFound(u64),
    /// Failed to construct [L2BlockInfo] from the block and genesis.
    #[error("Failed to construct L2BlockInfo from block {0} and genesis")]
    L2BlockInfoConstruction(u64),
    /// Failed to decode an [OpBlock] from the raw block.
    #[error("Failed to decode OpBlock from raw block {0}")]
    OpBlockDecode(u64),
    /// Failed to convert the block into a [SystemConfig].
    #[error("Failed to convert block {0} into SystemConfig")]
    SystemConfigConversion(u64),
}

impl From<AlloyL2ChainProviderError> for PipelineErrorKind {
    fn from(e: AlloyL2ChainProviderError) -> Self {
        match e {
            AlloyL2ChainProviderError::BlockNotFound(_) => {
                PipelineErrorKind::Temporary(PipelineError::Provider("block not found".to_string()))
            }
            AlloyL2ChainProviderError::L2BlockInfoConstruction(_) => PipelineErrorKind::Temporary(
                PipelineError::Provider("l2 block info construction failed".to_string()),
            ),
            AlloyL2ChainProviderError::OpBlockDecode(_) => PipelineErrorKind::Temporary(
                PipelineError::Provider("op block decode failed".to_string()),
            ),
            AlloyL2ChainProviderError::SystemConfigConversion(_) => PipelineErrorKind::Temporary(
                PipelineError::Provider("system config conversion failed".to_string()),
            ),
        }
    }
}

#[async_trait]
impl BatchValidationProvider for AlloyL2ChainProvider {
    type Error = AlloyL2ChainProviderError;
    type Transaction = OpTxEnvelope;

    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo, Self::Error> {
        if let Some(l2_block_info) = self.l2_block_info_by_number_cache.get(&number) {
            return Ok(*l2_block_info);
        }

        let block = match self.block_by_number(number).await {
            Ok(p) => p,
            Err(_) => {
                return Err(AlloyL2ChainProviderError::BlockNotFound(number));
            }
        };
        let l2_block_info =
            match L2BlockInfo::from_block_and_genesis(&block, &self.rollup_config.genesis) {
                Ok(b) => b,
                Err(_) => {
                    return Err(AlloyL2ChainProviderError::L2BlockInfoConstruction(number));
                }
            };
        self.l2_block_info_by_number_cache.put(number, l2_block_info);
        Ok(l2_block_info)
    }

    async fn block_by_number(&mut self, number: u64) -> Result<OpBlock, Self::Error> {
        if let Some(block) = self.block_by_number_cache.get(&number) {
            return Ok(block.clone());
        }

        let raw_block: TransportResult<Bytes> =
            self.inner.raw_request("debug_getRawBlock".into(), [U64::from(number)]).await;
        let raw_block: Bytes = match raw_block {
            Ok(b) => b,
            Err(_) => {
                return Err(AlloyL2ChainProviderError::BlockNotFound(number));
            }
        };
        let block = match OpBlock::decode(&mut raw_block.as_ref()) {
            Ok(b) => b,
            Err(_) => {
                return Err(AlloyL2ChainProviderError::OpBlockDecode(number));
            }
        };
        self.block_by_number_cache.put(number, block.clone());
        Ok(block)
    }
}

#[async_trait]
impl L2ChainProvider for AlloyL2ChainProvider {
    type Error = AlloyL2ChainProviderError;

    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> Result<SystemConfig, <Self as BatchValidationProvider>::Error> {
        if let Some(system_config) = self.system_config_by_number_cache.get(&number) {
            return Ok(*system_config);
        }

        let block = match self.block_by_number(number).await {
            Ok(e) => e,
            Err(_) => {
                return Err(AlloyL2ChainProviderError::BlockNotFound(number));
            }
        };
        let sys_config = match to_system_config(&block, &rollup_config) {
            Ok(s) => s,
            Err(_) => {
                return Err(AlloyL2ChainProviderError::SystemConfigConversion(number));
            }
        };
        self.system_config_by_number_cache.put(number, sys_config);
        Ok(sys_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_alloy_l2_chain_provider_latest_block_number() {
        let mut provider = AlloyL2ChainProvider::new_http(
            "https://docs-demo.quiknode.pro/".try_into().unwrap(),
            Arc::new(RollupConfig::default()),
        );
        let number = provider.latest_block_number().await.unwrap();
        assert!(number > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_alloy_l2_chain_provider_chain_id() {
        let mut provider = AlloyL2ChainProvider::new_http(
            "https://docs-demo.quiknode.pro/".try_into().unwrap(),
            Arc::new(RollupConfig::default()),
        );
        let chain_id = provider.chain_id().await.unwrap();
        assert_eq!(chain_id, 1);
    }
}
