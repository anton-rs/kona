//! Contains logic to validate derivation pipeline outputs.

use crate::types::{L2AttributesWithParent, L2PayloadAttributes, RawTransaction};
use alloc::{boxed::Box, vec};
use alloy_provider::{Provider, ReqwestProvider};
use alloy_rpc_types::{Block, BlockNumberOrTag, BlockTransactions};
use alloy_transport::TransportResult;
use anyhow::Result;
use async_trait::async_trait;

/// Validator
///
/// The validator trait describes the interface for validating the derivation outputs.
#[async_trait]
pub trait Validator {
    /// Validates the given [`L2AttributesWithParent`].
    async fn validate(&self, attributes: &L2AttributesWithParent) -> bool;
}

/// OnlineValidator
///
/// Validates the [`L2AttributesWithParent`] by fetching the associated L2 block from
/// a trusted L2 RPC and constructing the L2 Attributes from the block.
#[derive(Debug, Clone)]
pub struct OnlineValidator {
    /// The L2 provider.
    provider: ReqwestProvider,
}

impl OnlineValidator {
    /// Creates a new `OnlineValidator`.
    pub fn new(provider: ReqwestProvider) -> Self {
        Self { provider }
    }

    /// Creates a new [OnlineValidator] from the provided [reqwest::Url].
    pub fn new_http(url: reqwest::Url) -> Self {
        let inner = ReqwestProvider::new_http(url);
        Self::new(inner)
    }

    /// Fetches Transactions from the L2 provider.
    pub(crate) async fn get_block(&self, tag: BlockNumberOrTag) -> Result<Block> {
        let method = alloc::borrow::Cow::Borrowed("eth_getBlockByNumber");
        let block: TransportResult<Block> = self.provider.raw_request(method, (tag, true)).await;
        let block = block.map_err(|e| anyhow::anyhow!(e))?;
        Ok(block)
    }

    /// Gets the payload for the specified [BlockNumberOrTag].
    pub(crate) async fn get_payload(&self, tag: BlockNumberOrTag) -> Result<L2PayloadAttributes> {
        // TODO: we can't use the provider's get_block_by_number here because
        // upstream alloy will return Mainnet Ethereum transactions and not work for Optimism
        // Deposit Transactions.
        let block = self.get_block(tag).await?;
        let transactions = match block.transactions {
            BlockTransactions::Full(txns) => txns,
            _ => {
                return Err(anyhow::anyhow!("Block {tag} missing full transactions"));
            }
        };
        let transactions = transactions
            .iter()
            .map(|tx| {
                serde_json::to_vec(&tx)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize transaction: {:?}", e))
                    .map(RawTransaction::from)
            })
            .collect::<Result<alloc::vec::Vec<RawTransaction>>>()?;
        Ok(L2PayloadAttributes {
            timestamp: block.header.timestamp,
            prev_randao: block.header.mix_hash.unwrap_or_default(),
            fee_recipient: block.header.miner,
            // Withdrawals on optimism are always empty
            withdrawals: Some(vec![]),
            parent_beacon_block_root: Some(block.header.parent_hash),
            transactions,
            no_tx_pool: false,
            gas_limit: Some(block.header.gas_limit as u64),
        })
    }
}

#[async_trait]
impl Validator for OnlineValidator {
    async fn validate(&self, attributes: &L2AttributesWithParent) -> bool {
        let expected = attributes.parent.block_info.number + 1;
        let tag = BlockNumberOrTag::from(expected);
        let payload = self.get_payload(tag).await.unwrap();
        attributes.attributes == payload
    }
}
