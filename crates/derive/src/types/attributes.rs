//! Contains Payload Attribute Types.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{block::L2BlockInfo, payload::Withdrawals, RawTransaction};
use alloc::vec::Vec;
use alloy_primitives::{Address, B256};

/// Payload attributes.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PayloadAttributes {
    /// Value for the timestamp field of the new payload.
    #[cfg_attr(feature = "serde", serde(rename = "timestamp"))]
    pub timestamp: u64,
    /// Value for the random field of the new payload.
    #[cfg_attr(feature = "serde", serde(rename = "prevRandao"))]
    pub prev_randao: B256,
    /// Suggested value for the coinbase field of the new payload.
    #[cfg_attr(feature = "serde", serde(rename = "suggestedFeeRecipient"))]
    pub fee_recipient: Address,
    /// Withdrawals to include into the block -- should be nil or empty depending on Shanghai
    /// enablement.
    #[cfg_attr(feature = "serde", serde(rename = "withdrawals"))]
    pub withdrawals: Option<Withdrawals>,
    /// Parent beacon block root optional extension in Dencun.
    #[cfg_attr(feature = "serde", serde(rename = "parentBeaconBlockRoot"))]
    pub parent_beacon_block_root: Option<B256>,

    // Optimism additions.
    /// Transactions to force into the block (always at the start of the transactions list).
    #[cfg_attr(feature = "serde", serde(rename = "transactions"))]
    pub transactions: Vec<RawTransaction>,
    /// NoTxPool to disable adding any transactions from the transaction-pool.
    #[cfg_attr(feature = "serde", serde(rename = "noTxPool"))]
    pub no_tx_pool: bool,
    /// GasLimit override.
    #[cfg_attr(feature = "serde", serde(rename = "gasLimit"))]
    pub gas_limit: Option<u64>,
}

/// Payload Attributes with parent block reference.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributesWithParent {
    /// The payload attributes.
    pub attributes: PayloadAttributes,
    /// The parent block reference.
    pub parent: L2BlockInfo,
    /// Whether the current batch is the last in its span.
    pub is_last_in_span: bool,
}

impl AttributesWithParent {
    /// Create a new [AttributesWithParent] instance.
    pub fn new(attributes: PayloadAttributes, parent: L2BlockInfo, is_last_in_span: bool) -> Self {
        Self { attributes, parent, is_last_in_span }
    }

    /// Returns the payload attributes.
    pub fn attributes(&self) -> &PayloadAttributes {
        &self.attributes
    }

    /// Returns the parent block reference.
    pub fn parent(&self) -> &L2BlockInfo {
        &self.parent
    }

    /// Returns whether the current batch is the last in its span.
    pub fn is_last_in_span(&self) -> bool {
        self.is_last_in_span
    }
}
