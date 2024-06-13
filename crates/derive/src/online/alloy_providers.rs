//! This module contains concrete implementations of the data provider traits, using an alloy
//! provider on the backend.

use crate::{
    traits::{ChainProvider, L2ChainProvider},
    types::{
        Block, BlockInfo, L2BlockInfo, L2ExecutionPayloadEnvelope, OpBlock, RollupConfig,
        SystemConfig,
    },
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_consensus::{Header, Receipt, ReceiptWithBloom, TxEnvelope, TxType};
use alloy_primitives::{Bytes, B256, U64};
use alloy_provider::{Provider, ReqwestProvider};
use alloy_rlp::{Buf, Decodable};
use alloy_transport::TransportResult;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use core::num::NonZeroUsize;
use lru::LruCache;

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
}

#[async_trait]
impl ChainProvider for AlloyChainProvider {
    async fn header_by_hash(&mut self, hash: B256) -> Result<Header> {
        if let Some(header) = self.header_by_hash_cache.get(&hash) {
            return Ok(header.clone());
        }

        let raw_header: TransportResult<Bytes> =
            self.inner.raw_request("debug_getRawHeader".into(), [hash]).await;
        let raw_header: Bytes = raw_header.map_err(|e| anyhow!(e))?;
        Header::decode(&mut raw_header.as_ref()).map_err(|e| anyhow!(e))
    }

    async fn block_info_by_number(&mut self, number: u64) -> Result<BlockInfo> {
        if let Some(block_info) = self.block_info_by_number_cache.get(&number) {
            return Ok(*block_info);
        }

        let raw_header: TransportResult<Bytes> =
            self.inner.raw_request("debug_getRawHeader".into(), [U64::from(number)]).await;
        let raw_header: Bytes = raw_header.map_err(|e| anyhow!(e))?;
        let header = Header::decode(&mut raw_header.as_ref()).map_err(|e| anyhow!(e))?;

        let block_info = BlockInfo {
            hash: header.hash_slow(),
            number,
            parent_hash: header.parent_hash,
            timestamp: header.timestamp,
        };
        self.block_info_by_number_cache.put(number, block_info);
        Ok(block_info)
    }

    async fn receipts_by_hash(&mut self, hash: B256) -> Result<Vec<Receipt>> {
        if let Some(receipts) = self.receipts_by_hash_cache.get(&hash) {
            return Ok(receipts.clone());
        }

        let raw_receipts: TransportResult<Vec<Bytes>> =
            self.inner.raw_request("debug_getRawReceipts".into(), [hash]).await;
        let raw_receipts: Vec<Bytes> = raw_receipts.map_err(|e| anyhow!(e))?;

        let receipts = raw_receipts
            .iter()
            .map(|r| {
                let r = &mut r.as_ref();

                // Skip the transaction type byte if it exists
                if !r.is_empty() && r[0] <= TxType::Eip4844 as u8 {
                    r.advance(1);
                }

                Ok(ReceiptWithBloom::decode(r).map_err(|e| anyhow!(e))?.receipt)
            })
            .collect::<Result<Vec<_>>>()?;
        self.receipts_by_hash_cache.put(hash, receipts.clone());
        Ok(receipts)
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        if let Some(block_info_and_txs) = self.block_info_and_transactions_by_hash_cache.get(&hash)
        {
            return Ok(block_info_and_txs.clone());
        }

        let raw_block: TransportResult<Bytes> =
            self.inner.raw_request("debug_getRawBlock".into(), [hash]).await;
        let raw_block: Bytes = raw_block.map_err(|e| anyhow!(e))?;
        let block = Block::decode(&mut raw_block.as_ref()).map_err(|e| anyhow!(e))?;

        let block_info = BlockInfo {
            hash: block.header.hash_slow(),
            number: block.header.number,
            parent_hash: block.header.parent_hash,
            timestamp: block.header.timestamp,
        };
        self.block_info_and_transactions_by_hash_cache.put(hash, (block_info, block.body.clone()));
        Ok((block_info, block.body))
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
    /// `payload_by_number` LRU cache.
    payload_by_number_cache: LruCache<u64, L2ExecutionPayloadEnvelope>,
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
            payload_by_number_cache: LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()),
            l2_block_info_by_number_cache: LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()),
            system_config_by_number_cache: LruCache::new(NonZeroUsize::new(CACHE_SIZE).unwrap()),
        }
    }

    /// Creates a new [AlloyL2ChainProvider] from the provided [reqwest::Url].
    pub fn new_http(url: reqwest::Url, rollup_config: Arc<RollupConfig>) -> Self {
        let inner = ReqwestProvider::new_http(url);
        Self::new(inner, rollup_config)
    }
}

#[async_trait]
impl L2ChainProvider for AlloyL2ChainProvider {
    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo> {
        if let Some(l2_block_info) = self.l2_block_info_by_number_cache.get(&number) {
            return Ok(*l2_block_info);
        }

        let payload = self.payload_by_number(number).await?;
        let l2_block_info = payload.to_l2_block_ref(self.rollup_config.as_ref())?;
        self.l2_block_info_by_number_cache.put(number, l2_block_info);
        Ok(l2_block_info)
    }

    async fn payload_by_number(&mut self, number: u64) -> Result<L2ExecutionPayloadEnvelope> {
        if let Some(payload) = self.payload_by_number_cache.get(&number) {
            return Ok(payload.clone());
        }

        let raw_block: TransportResult<Bytes> =
            self.inner.raw_request("debug_getRawBlock".into(), [U64::from(number)]).await;
        let raw_block: Bytes = raw_block.map_err(|e| anyhow!(e))?;
        let block = OpBlock::decode(&mut raw_block.as_ref()).map_err(|e| anyhow!(e))?;
        let payload_envelope: L2ExecutionPayloadEnvelope = block.into();

        self.payload_by_number_cache.put(number, payload_envelope.clone());
        Ok(payload_envelope)
    }

    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> Result<SystemConfig> {
        if let Some(system_config) = self.system_config_by_number_cache.get(&number) {
            return Ok(system_config.clone());
        }

        let envelope = self.payload_by_number(number).await?;
        envelope.to_system_config(&rollup_config)
    }
}
