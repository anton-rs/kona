//! Providers that use alloy provider types on the backend.

use crate::AlloyProviderError;
use alloy_consensus::{Block, Header, Receipt, ReceiptWithBloom, TxEnvelope, TxType};
use alloy_primitives::{Bytes, B256, U64};
use alloy_provider::{Provider, ReqwestProvider};
use alloy_rlp::{Buf, Decodable};
use alloy_transport::{RpcError, TransportErrorKind};
use async_trait::async_trait;
use kona_derive::traits::{ChainProvider, L2ChainProvider};
use lru::LruCache;
use op_alloy_consensus::OpBlock;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{to_system_config, BatchValidationProvider, BlockInfo, L2BlockInfo};
use std::{boxed::Box, num::NonZeroUsize, sync::Arc, vec::Vec};

const CACHE_SIZE: usize = 16;

/// The [AlloyChainProvider] is a concrete implementation of the [ChainProvider] trait, providing
/// data over Ethereum JSON-RPC using an alloy provider as the backend.
///
/// **Note**:
/// This provider fetches data using the `debug_getRawHeader`, `debug_getRawReceipts`, and
/// `debug_getRawBlock` methods. The RPC must support this namespace.
#[derive(Debug, Clone)]
pub struct AlloyChainProvider {
    /// The inner Ethereum JSON-RPC provider.
    inner: ReqwestProvider,
    /// `header_by_hash` LRU cache.
    header_by_hash_cache: LruCache<B256, Header>,
    /// `block_info_by_number` LRU cache.
    block_info_by_number_cache: LruCache<u64, BlockInfo>,
    /// `block_info_by_number` LRU cache.
    receipts_by_hash_cache: LruCache<B256, Vec<Receipt>>,
    /// `block_info_and_transactions_by_hash` LRU cache.
    block_info_and_transactions_by_hash_cache: LruCache<B256, (BlockInfo, Vec<TxEnvelope>)>,
}

impl AlloyChainProvider {
    /// Creates a new [AlloyChainProvider] with the given alloy provider.
    pub fn new(inner: ReqwestProvider) -> Self {
        Self {
            inner,
            header_by_hash_cache: LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()),
            block_info_by_number_cache: LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()),
            receipts_by_hash_cache: LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()),
            block_info_and_transactions_by_hash_cache: LruCache::new(
                NonZeroUsize::new(CACHE_SIZE).unwrap(),
            ),
        }
    }

    /// Creates a new [AlloyChainProvider] from the provided [reqwest::Url].
    pub fn new_http(url: reqwest::Url) -> Self {
        let inner = ReqwestProvider::new_http(url);
        Self::new(inner)
    }

    /// Returns the latest L2 block number.
    pub async fn latest_block_number(&mut self) -> Result<u64, RpcError<TransportErrorKind>> {
        self.inner.get_block_number().await
    }

    /// Returns the chain ID.
    pub async fn chain_id(&mut self) -> Result<u64, RpcError<TransportErrorKind>> {
        self.inner.get_chain_id().await
    }
}

#[async_trait]
impl ChainProvider for AlloyChainProvider {
    type Error = AlloyProviderError;

    async fn header_by_hash(&mut self, hash: B256) -> Result<Header, Self::Error> {
        if let Some(header) = self.header_by_hash_cache.get(&hash) {
            return Ok(header.clone());
        }

        let raw_header: Bytes = self
            .inner
            .raw_request("debug_getRawHeader".into(), [hash])
            .await
            .map_err(AlloyProviderError::Rpc)?;
        let header = Header::decode(&mut raw_header.as_ref()).map_err(AlloyProviderError::Rlp)?;

        self.header_by_hash_cache.put(hash, header.clone());
        Ok(header)
    }

    async fn block_info_by_number(&mut self, number: u64) -> Result<BlockInfo, Self::Error> {
        if let Some(block_info) = self.block_info_by_number_cache.get(&number) {
            return Ok(*block_info);
        }

        let raw_header: Bytes = self
            .inner
            .raw_request("debug_getRawHeader".into(), [U64::from(number)])
            .await
            .map_err(AlloyProviderError::Rpc)?;
        let header = Header::decode(&mut raw_header.as_ref()).map_err(AlloyProviderError::Rlp)?;

        let block_info = BlockInfo {
            hash: header.hash_slow(),
            number,
            parent_hash: header.parent_hash,
            timestamp: header.timestamp,
        };
        self.block_info_by_number_cache.put(number, block_info);
        Ok(block_info)
    }

    async fn receipts_by_hash(&mut self, hash: B256) -> Result<Vec<Receipt>, Self::Error> {
        if let Some(receipts) = self.receipts_by_hash_cache.get(&hash) {
            return Ok(receipts.clone());
        }

        let raw_receipts: Vec<Bytes> = self
            .inner
            .raw_request("debug_getRawReceipts".into(), [hash])
            .await
            .map_err(AlloyProviderError::Rpc)?;
        let receipts = raw_receipts
            .iter()
            .map(|r| {
                let r = &mut r.as_ref();

                // Skip the transaction type byte if it exists
                if !r.is_empty() && r[0] <= TxType::Eip4844 as u8 {
                    r.advance(1);
                }

                Ok(ReceiptWithBloom::decode(r).map_err(AlloyProviderError::Rlp)?.receipt)
            })
            .collect::<Result<Vec<_>, Self::Error>>()?;
        self.receipts_by_hash_cache.put(hash, receipts.clone());
        Ok(receipts)
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>), Self::Error> {
        if let Some(block_info_and_txs) = self.block_info_and_transactions_by_hash_cache.get(&hash)
        {
            return Ok(block_info_and_txs.clone());
        }

        let raw_block: Bytes = self
            .inner
            .raw_request("debug_getRawBlock".into(), [hash])
            .await
            .map_err(AlloyProviderError::Rpc)?;
        let block: Block<TxEnvelope> =
            Block::decode(&mut raw_block.as_ref()).map_err(AlloyProviderError::Rlp)?;

        let block_info = BlockInfo {
            hash: block.header.hash_slow(),
            number: block.header.number,
            parent_hash: block.header.parent_hash,
            timestamp: block.header.timestamp,
        };
        self.block_info_and_transactions_by_hash_cache
            .put(hash, (block_info, block.body.transactions.clone()));
        Ok((block_info, block.body.transactions))
    }
}

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
    block_by_number_cache: LruCache<u64, OpBlock>,
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

#[async_trait]
impl BatchValidationProvider for AlloyL2ChainProvider {
    type Error = AlloyProviderError;

    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo, Self::Error> {
        if let Some(l2_block_info) = self.l2_block_info_by_number_cache.get(&number) {
            return Ok(*l2_block_info);
        }

        let block = self.block_by_number(number).await?;
        let l2_block_info =
            L2BlockInfo::from_block_and_genesis(&block, &self.rollup_config.genesis)
                .map_err(AlloyProviderError::BlockInfo)?;
        self.l2_block_info_by_number_cache.put(number, l2_block_info);
        Ok(l2_block_info)
    }

    async fn block_by_number(&mut self, number: u64) -> Result<OpBlock, Self::Error> {
        if let Some(block) = self.block_by_number_cache.get(&number) {
            return Ok(block.clone());
        }

        let raw_block: Bytes = self
            .inner
            .raw_request("debug_getRawBlock".into(), [U64::from(number)])
            .await
            .map_err(AlloyProviderError::Rpc)?;
        let block = OpBlock::decode(&mut raw_block.as_ref()).map_err(AlloyProviderError::Rlp)?;
        self.block_by_number_cache.put(number, block.clone());
        Ok(block)
    }
}

#[async_trait]
impl L2ChainProvider for AlloyL2ChainProvider {
    type Error = AlloyProviderError;

    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> Result<SystemConfig, <Self as L2ChainProvider>::Error> {
        if let Some(system_config) = self.system_config_by_number_cache.get(&number) {
            return Ok(*system_config);
        }

        let block = self.block_by_number(number).await?;
        let sys_config = to_system_config(&block, &rollup_config)
            .map_err(AlloyProviderError::OpBlockConversion)?;
        self.system_config_by_number_cache.put(number, sys_config);
        Ok(sys_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_provider() -> ReqwestProvider {
        ReqwestProvider::new_http("https://docs-demo.quiknode.pro/".try_into().unwrap())
    }

    #[tokio::test]
    async fn test_alloy_chain_provider_latest_block_number() {
        let mut provider = AlloyChainProvider::new(default_provider());
        let number = provider.latest_block_number().await.unwrap();
        assert!(number > 0);
    }

    #[tokio::test]
    async fn test_alloy_chain_provider_chain_id() {
        let mut provider = AlloyChainProvider::new(default_provider());
        let chain_id = provider.chain_id().await.unwrap();
        assert_eq!(chain_id, 1);
    }

    #[tokio::test]
    async fn test_alloy_l2_chain_provider_latest_block_number() {
        let mut provider = AlloyL2ChainProvider::new_http(
            "https://docs-demo.quiknode.pro/".try_into().unwrap(),
            Arc::new(RollupConfig::default()),
        );
        let number = provider.latest_block_number().await.unwrap();
        assert!(number > 0);
    }

    #[tokio::test]
    async fn test_alloy_l2_chain_provider_chain_id() {
        let mut provider = AlloyL2ChainProvider::new_http(
            "https://docs-demo.quiknode.pro/".try_into().unwrap(),
            Arc::new(RollupConfig::default()),
        );
        let chain_id = provider.chain_id().await.unwrap();
        assert_eq!(chain_id, 1);
    }
}
