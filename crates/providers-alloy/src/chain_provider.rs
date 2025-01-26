//! Providers that use alloy provider types on the backend.

use alloy_consensus::{Block, Header, Receipt, ReceiptWithBloom, TxEnvelope, TxType};
use alloy_primitives::{Bytes, B256, U64};
use alloy_provider::{Provider, ReqwestProvider};
use alloy_rlp::{Buf, Decodable};
use alloy_transport::{RpcError, TransportErrorKind};
use async_trait::async_trait;
use kona_derive::{
    errors::{PipelineError, PipelineErrorKind},
    traits::ChainProvider,
};
use lru::LruCache;
use maili_protocol::BlockInfo;
use std::{boxed::Box, num::NonZeroUsize, vec::Vec};

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

/// An error for the [AlloyChainProvider].
#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum AlloyChainProviderError {
    /// Failed to fetch the raw header.
    #[error("Failed to fetch raw header for hash {0}")]
    RawHeaderFetch(B256),
    /// Failed to decode the raw header.
    #[error("Failed to decode raw header for hash {0}")]
    RawHeaderDecoding(B256),
    /// Failed to fetch the raw receipts.
    #[error("Failed to fetch raw receipts for hash {0}")]
    RawReceiptsFetch(B256),
    /// Failed to decode the raw receipts.
    #[error("Failed to decode raw receipts for hash {0}")]
    RawReceiptsDecoding(B256),
}

impl From<AlloyChainProviderError> for PipelineErrorKind {
    fn from(e: AlloyChainProviderError) -> Self {
        match e {
            AlloyChainProviderError::RawHeaderFetch(_) => PipelineErrorKind::Temporary(
                PipelineError::Provider("Failed to fetch raw header".to_string()),
            ),
            AlloyChainProviderError::RawHeaderDecoding(_) => PipelineErrorKind::Temporary(
                PipelineError::Provider("Failed to decode raw header".to_string()),
            ),
            AlloyChainProviderError::RawReceiptsFetch(_) => PipelineErrorKind::Temporary(
                PipelineError::Provider("Failed to fetch raw receipts".to_string()),
            ),
            AlloyChainProviderError::RawReceiptsDecoding(_) => PipelineErrorKind::Temporary(
                PipelineError::Provider("Failed to decode raw receipts".to_string()),
            ),
        }
    }
}

#[async_trait]
impl ChainProvider for AlloyChainProvider {
    type Error = AlloyChainProviderError;

    async fn header_by_hash(&mut self, hash: B256) -> Result<Header, Self::Error> {
        if let Some(header) = self.header_by_hash_cache.get(&hash) {
            return Ok(header.clone());
        }

        let raw_header: Bytes = self
            .inner
            .raw_request("debug_getRawHeader".into(), [hash])
            .await
            .map_err(|_| AlloyChainProviderError::RawHeaderFetch(hash))?;
        match Header::decode(&mut raw_header.as_ref()) {
            Ok(header) => {
                self.header_by_hash_cache.put(hash, header.clone());
                Ok(header)
            }
            Err(_) => Err(AlloyChainProviderError::RawHeaderDecoding(hash)),
        }
    }

    async fn block_info_by_number(&mut self, number: u64) -> Result<BlockInfo, Self::Error> {
        if let Some(block_info) = self.block_info_by_number_cache.get(&number) {
            return Ok(*block_info);
        }

        let raw_header: Bytes = self
            .inner
            .raw_request("debug_getRawHeader".into(), [U64::from(number)])
            .await
            .map_err(|_| AlloyChainProviderError::RawHeaderFetch(B256::default()))?;
        let header = match Header::decode(&mut raw_header.as_ref()) {
            Ok(h) => h,
            Err(_) => {
                return Err(AlloyChainProviderError::RawHeaderDecoding(B256::default()));
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
        if let Some(receipts) = self.receipts_by_hash_cache.get(&hash) {
            return Ok(receipts.clone());
        }

        let raw_receipts: Vec<Bytes> = self
            .inner
            .raw_request("debug_getRawReceipts".into(), [hash])
            .await
            .map_err(|_| AlloyChainProviderError::RawReceiptsFetch(hash))?;

        let receipts = raw_receipts
            .iter()
            .map(|r| {
                let r = &mut r.as_ref();

                // Skip the transaction type byte if it exists
                if !r.is_empty() && r[0] <= TxType::Eip4844 as u8 {
                    r.advance(1);
                }

                Ok(ReceiptWithBloom::decode(r)
                    .map_err(|_| AlloyChainProviderError::RawReceiptsDecoding(hash))?
                    .receipt)
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
            .map_err(|_| AlloyChainProviderError::RawHeaderFetch(hash))?;
        let block: Block<TxEnvelope> = match Block::decode(&mut raw_block.as_ref()) {
            Ok(b) => b,
            Err(_) => {
                return Err(AlloyChainProviderError::RawHeaderDecoding(hash));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_provider() -> ReqwestProvider {
        ReqwestProvider::new_http("https://docs-demo.quiknode.pro/".try_into().unwrap())
    }

    #[tokio::test]
    #[ignore]
    async fn test_alloy_chain_provider_latest_block_number() {
        let mut provider = AlloyChainProvider::new(default_provider());
        let number = provider.latest_block_number().await.unwrap();
        assert!(number > 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_alloy_chain_provider_chain_id() {
        let mut provider = AlloyChainProvider::new(default_provider());
        let chain_id = provider.chain_id().await.unwrap();
        assert_eq!(chain_id, 1);
    }
}
