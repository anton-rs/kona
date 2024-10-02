//! Providers that use alloy provider types on the backend.

use alloy_consensus::{Block, Header, Receipt, ReceiptWithBloom, TxEnvelope, TxType};
use alloy_primitives::{Bytes, B256, U64};
use alloy_provider::{Provider, ReqwestProvider};
use alloy_rlp::{Buf, Decodable};
use alloy_transport::{RpcError, TransportErrorKind, TransportResult};
use async_trait::async_trait;
use kona_providers::{to_system_config, ChainProvider, L2ChainProvider};
use lru::LruCache;
use op_alloy_consensus::OpBlock;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
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
    type Error = RpcError<TransportErrorKind>;

    async fn header_by_hash(&mut self, hash: B256) -> Result<Header, Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["chain_provider", "header_by_hash"]);
        crate::timer!(START, PROVIDER_RESPONSE_TIME, &["chain_provider", "header_by_hash"], timer);
        if let Some(header) = self.header_by_hash_cache.get(&hash) {
            return Ok(header.clone());
        }

        let raw_header: TransportResult<Bytes> =
            self.inner.raw_request("debug_getRawHeader".into(), [hash]).await;
        let raw_header: Bytes = match raw_header {
            Ok(b) => b,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["chain_provider", "header_by_hash", "debug_getRawHeader"]
                );
                return Err(e);
            }
        };
        match Header::decode(&mut raw_header.as_ref()) {
            Ok(header) => {
                self.header_by_hash_cache.put(hash, header.clone());
                Ok(header)
            }
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["chain_provider", "header_by_hash", "decode"]);
                Err(RpcError::LocalUsageError(Box::new(e)))
            }
        }
    }

    async fn block_info_by_number(&mut self, number: u64) -> Result<BlockInfo, Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["chain_provider", "block_info_by_number"]);
        crate::timer!(
            START,
            PROVIDER_RESPONSE_TIME,
            &["chain_provider", "block_info_by_number"],
            timer
        );
        if let Some(block_info) = self.block_info_by_number_cache.get(&number) {
            return Ok(*block_info);
        }

        let raw_header: TransportResult<Bytes> =
            self.inner.raw_request("debug_getRawHeader".into(), [U64::from(number)]).await;
        let raw_header: Bytes = match raw_header {
            Ok(b) => b,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["chain_provider", "block_info_by_number", "debug_getRawHeader"]
                );
                return Err(e);
            }
        };
        let header = match Header::decode(&mut raw_header.as_ref()) {
            Ok(h) => h,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["chain_provider", "block_info_by_number", "decode"]);
                return Err(RpcError::LocalUsageError(Box::new(e)));
            }
        };

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
        crate::inc!(PROVIDER_CALLS, &["chain_provider", "receipts_by_hash"]);
        crate::timer!(
            START,
            PROVIDER_RESPONSE_TIME,
            &["chain_provider", "receipts_by_hash"],
            timer
        );
        if let Some(receipts) = self.receipts_by_hash_cache.get(&hash) {
            return Ok(receipts.clone());
        }

        let raw_receipts: TransportResult<Vec<Bytes>> =
            self.inner.raw_request("debug_getRawReceipts".into(), [hash]).await;
        let raw_receipts: Vec<Bytes> = match raw_receipts {
            Ok(r) => r,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["chain_provider", "receipts_by_hash", "debug_getRawReceipts"]
                );
                return Err(e);
            }
        };

        let receipts = match raw_receipts
            .iter()
            .map(|r| {
                let r = &mut r.as_ref();

                // Skip the transaction type byte if it exists
                if !r.is_empty() && r[0] <= TxType::Eip4844 as u8 {
                    r.advance(1);
                }

                Ok(ReceiptWithBloom::decode(r)
                    .map_err(|e| RpcError::LocalUsageError(Box::new(e)))?
                    .receipt)
            })
            .collect::<Result<Vec<_>, Self::Error>>()
        {
            Ok(r) => r,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["chain_provider", "receipts_by_hash", "decode"]);
                return Err(e);
            }
        };
        self.receipts_by_hash_cache.put(hash, receipts.clone());
        Ok(receipts)
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>), Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["chain_provider", "block_info_and_transactions_by_hash"]);
        crate::timer!(
            START,
            PROVIDER_RESPONSE_TIME,
            &["chain_provider", "block_info_and_transactions_by_hash"],
            timer
        );
        if let Some(block_info_and_txs) = self.block_info_and_transactions_by_hash_cache.get(&hash)
        {
            return Ok(block_info_and_txs.clone());
        }

        let raw_block: TransportResult<Bytes> =
            self.inner.raw_request("debug_getRawBlock".into(), [hash]).await;
        let raw_block: Bytes = match raw_block {
            Ok(b) => b,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["chain_provider", "block_info_and_transactions_by_hash", "debug_getRawBlock"]
                );
                return Err(e);
            }
        };
        let block = match Block::decode(&mut raw_block.as_ref()) {
            Ok(b) => b,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["chain_provider", "block_info_and_transactions_by_hash", "decode"]
                );
                return Err(RpcError::LocalUsageError(Box::new(e)));
            }
        };

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
impl L2ChainProvider for AlloyL2ChainProvider {
    type Error = RpcError<TransportErrorKind>;

    async fn l2_block_info_by_number(&mut self, number: u64) -> Result<L2BlockInfo, Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["l2_chain_provider", "l2_block_info_by_number"]);
        crate::timer!(
            START,
            PROVIDER_RESPONSE_TIME,
            &["l2_chain_provider", "l2_block_info_by_number"],
            timer
        );
        if let Some(l2_block_info) = self.l2_block_info_by_number_cache.get(&number) {
            return Ok(*l2_block_info);
        }

        let block = match self.block_by_number(number).await {
            Ok(p) => p,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["l2_chain_provider", "l2_block_info_by_number", "block_by_number"]
                );
                return Err(e);
            }
        };
        let l2_block_info =
            match L2BlockInfo::from_block_and_genesis(&block, &self.rollup_config.genesis) {
                Ok(b) => b,
                Err(e) => {
                    crate::timer!(DISCARD, timer);
                    crate::inc!(
                        PROVIDER_ERRORS,
                        &["l2_chain_provider", "l2_block_info_by_number", "from_block_and_genesis"]
                    );
                    return Err(RpcError::LocalUsageError(Box::new(e)));
                }
            };
        self.l2_block_info_by_number_cache.put(number, l2_block_info);
        Ok(l2_block_info)
    }

    async fn block_by_number(&mut self, number: u64) -> Result<OpBlock, Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["l2_chain_provider", "block_by_number"]);
        crate::timer!(
            START,
            PROVIDER_RESPONSE_TIME,
            &["l2_chain_provider", "block_by_number"],
            timer
        );
        if let Some(block) = self.block_by_number_cache.get(&number) {
            return Ok(block.clone());
        }

        let raw_block: TransportResult<Bytes> =
            self.inner.raw_request("debug_getRawBlock".into(), [U64::from(number)]).await;
        let raw_block: Bytes = match raw_block {
            Ok(b) => b,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["l2_chain_provider", "block_by_number", "debug_getRawBlock"]
                );
                return Err(e);
            }
        };
        let block = match OpBlock::decode(&mut raw_block.as_ref()) {
            Ok(b) => b,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(PROVIDER_ERRORS, &["l2_chain_provider", "block_by_number", "decode"]);
                return Err(RpcError::LocalUsageError(Box::new(e)));
            }
        };
        self.block_by_number_cache.put(number, block.clone());
        Ok(block)
    }

    async fn system_config_by_number(
        &mut self,
        number: u64,
        rollup_config: Arc<RollupConfig>,
    ) -> Result<SystemConfig, Self::Error> {
        crate::inc!(PROVIDER_CALLS, &["l2_chain_provider", "system_config_by_number"]);
        crate::timer!(
            START,
            PROVIDER_RESPONSE_TIME,
            &["l2_chain_provider", "system_config_by_number"],
            timer
        );
        if let Some(system_config) = self.system_config_by_number_cache.get(&number) {
            return Ok(*system_config);
        }

        let block = match self.block_by_number(number).await {
            Ok(e) => e,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["l2_chain_provider", "system_config_by_number", "block_by_number"]
                );
                return Err(e);
            }
        };
        let sys_config = match to_system_config(&block, &rollup_config) {
            Ok(s) => s,
            Err(e) => {
                crate::timer!(DISCARD, timer);
                crate::inc!(
                    PROVIDER_ERRORS,
                    &["l2_chain_provider", "system_config_by_number", "to_system_config"]
                );
                return Err(RpcError::LocalUsageError(Box::new(e)));
            }
        };
        self.system_config_by_number_cache.put(number, sys_config);
        Ok(sys_config)
    }
}
