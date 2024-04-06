//! This module contains concrete implementations of the data provider traits, using an alloy
//! provider on the backend.

use crate::{
    traits::ChainProvider,
    types::{Block, BlockInfo},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_consensus::{Header, Receipt, ReceiptWithBloom, TxEnvelope, TxType};
use alloy_primitives::{Bytes, B256, U64};
use alloy_provider::Provider;
use alloy_rlp::{Buf, Decodable};
use alloy_transport_http::Http;
use anyhow::{anyhow, Result};
use async_trait::async_trait;

/// The [AlloyChainProvider] is a concrete implementation of the [ChainProvider] trait, providing
/// data over Ethereum JSON-RPC using an alloy provider as the backend.
///
/// **Note**:
/// This provider fetches data using the `debug_getRawHeader`, `debug_getRawReceipts`, and
/// `debug_getRawBlock` methods. The RPC must support this namespace.
#[derive(Debug)]
pub struct AlloyChainProvider<T: Provider<Http<reqwest::Client>>> {
    inner: T,
}

impl<T: Provider<Http<reqwest::Client>>> AlloyChainProvider<T> {
    /// Creates a new [AlloyChainProvider] with the given alloy provider.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<T: Provider<Http<reqwest::Client>>> ChainProvider for AlloyChainProvider<T> {
    /// Returns the block at the given number, or an error if the block does not exist in the data
    /// source.
    async fn block_info_by_number(&self, number: u64) -> Result<BlockInfo> {
        let raw_header: Bytes = self
            .inner
            .client()
            .request("debug_getRawHeader", [U64::from(number)])
            .await
            .map_err(|e| anyhow!(e))?;
        let header = Header::decode(&mut raw_header.as_ref()).map_err(|e| anyhow!(e))?;

        Ok(BlockInfo {
            hash: header.hash_slow(),
            number,
            parent_hash: header.parent_hash,
            timestamp: header.timestamp,
        })
    }

    /// Returns all receipts in the block with the given hash, or an error if the block does not
    /// exist in the data source.
    async fn receipts_by_hash(&self, hash: B256) -> Result<Vec<Receipt>> {
        let raw_receipts: Vec<Bytes> = self
            .inner
            .client()
            .request("debug_getRawReceipts", [hash])
            .await
            .map_err(|e| anyhow!(e))?;

        raw_receipts
            .iter()
            .map(|r| {
                let r = &mut r.as_ref();

                // Skip the transaction type byte if it exists
                if !r.is_empty() && r[0] <= TxType::Eip4844 as u8 {
                    r.advance(1);
                }

                Ok(ReceiptWithBloom::decode(r).map_err(|e| anyhow!(e))?.receipt)
            })
            .collect::<Result<Vec<_>>>()
    }

    /// Returns the [BlockInfo] and list of [TxEnvelope]s from the given block hash.
    async fn block_info_and_transactions_by_hash(
        &self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        let raw_block: Bytes = self
            .inner
            .client()
            .request("debug_getRawBlock", [hash])
            .await
            .map_err(|e| anyhow!(e))?;
        let block = Block::decode(&mut raw_block.as_ref()).map_err(|e| anyhow!(e))?;

        let block_info = BlockInfo {
            hash: block.header.hash_slow(),
            number: block.header.number,
            parent_hash: block.header.parent_hash,
            timestamp: block.header.timestamp,
        };
        Ok((block_info, block.body))
    }
}
