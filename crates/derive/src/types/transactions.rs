#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use alloc::vec::Vec;
use alloy_primitives::{hex, Address, Bytes, B256, U256, U64};
use alloy_rlp::{Decodable, Error as DecoderError};

/// Serde helper for default address
#[allow(dead_code)]
const fn default_address() -> Address {
    Address::ZERO
}

/// A Transaction
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Transaction {
    /// The transaction hash
    pub hash: B256,
    /// The transaction nonce
    pub nonce: U256,
    /// The block hash
    #[cfg_attr(feature = "serde", serde(default, rename = "blockHash"))]
    pub block_hash: Option<B256>,
    /// The block number
    #[cfg_attr(feature = "serde", serde(default, rename = "blockNumber"))]
    pub block_number: Option<U256>,
    /// The transaction index
    #[cfg_attr(feature = "serde", serde(default, rename = "transactionIndex"))]
    pub transaction_index: Option<U256>,
    /// The transaction sender
    #[cfg_attr(feature = "serde", serde(default = "default_address"))]
    pub from: Address,
    /// The destination address of the transaction
    #[cfg_attr(feature = "serde", serde(default))]
    pub to: Option<Address>,
    /// The transaction value
    pub value: U256,
    /// The gas price
    #[cfg_attr(feature = "serde", serde(rename = "gasPrice"))]
    pub gas_price: Option<U256>,
    /// The amount of gas used
    pub gas: U256,
    /// The input data
    pub input: Bytes,
    /// `v` value of the transaction signature
    pub v: U64,
    /// `r` value of the transaction signature
    pub r: U256,
    /// `s` value of the transaction signature
    pub s: U256,
    /// The source hash
    pub source_hash: B256,
    /// If the transaction mints
    pub mint: Option<U256>,
    /// If the transaction is a system transaction
    pub is_system_tx: bool,
    /// The transaction type
    #[cfg_attr(
        feature = "serde",
        serde(rename = "type", default, skip_serializing_if = "Option::is_none")
    )]
    pub transaction_type: Option<U64>,
    /// The max priority fee per gas
    #[cfg_attr(
        feature = "serde",
        serde(
            rename = "accessList",
            default,
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub max_priority_fee_per_gas: Option<U256>,
    /// The max fee per gas
    #[cfg_attr(
        feature = "serde",
        serde(
            rename = "maxFeePerGas",
            default,
            skip_serializing_if = "Option::is_none"
        )
    )]
    pub max_fee_per_gas: Option<U256>,
    /// The chain id
    #[cfg_attr(
        feature = "serde",
        serde(rename = "chainId", default, skip_serializing_if = "Option::is_none")
    )]
    pub chain_id: Option<U256>,
}

/// A raw transaction
#[derive(Clone, PartialEq, Eq)]
pub struct RawTransaction(pub Vec<u8>);

impl From<Vec<u8>> for RawTransaction {
    fn from(tx: Vec<u8>) -> Self {
        Self(tx)
    }
}

impl Decodable for RawTransaction {
    fn decode(rlp: &mut &[u8]) -> Result<Self, DecoderError> {
        let tx_bytes: Vec<u8> = Decodable::decode(rlp)?;
        Ok(Self(tx_bytes))
    }
}

impl core::fmt::Debug for RawTransaction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{}", hex::encode(&self.0))
    }
}

#[cfg(feature = "serde")]
impl Serialize for RawTransaction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&alloc::format!("0x{}", hex::encode(&self.0)))
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for RawTransaction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let tx: alloc::string::String = serde::Deserialize::deserialize(deserializer)?;
        let tx = tx.strip_prefix("0x").unwrap_or(&tx);
        Ok(RawTransaction(hex::decode(tx).map_err(D::Error::custom)?))
    }
}
