//! This module contains the various Block types.

use alloc::vec::Vec;
use alloy_consensus::{Header, TxEnvelope};
use alloy_eips::eip4895::Withdrawal;
use alloy_primitives::B256;
use alloy_rlp::{RlpDecodable, RlpEncodable};
use op_alloy_consensus::OpTxEnvelope;
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{
    block_info::DecodeError, BlockInfo, L1BlockInfoBedrock, L1BlockInfoEcotone, L1BlockInfoTx,
    L2BlockInfo,
};
use thiserror::Error;

/// Ethereum full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
///
/// Taken from [reth-primitives](https://github.com/paradigmxyz/reth)
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct Block {
    /// Block header.
    pub header: Header,
    /// Transactions in this block.
    pub body: Vec<TxEnvelope>,
    /// Ommers/uncles header.
    pub ommers: Vec<Header>,
    /// Block withdrawals.
    pub withdrawals: Option<Vec<Withdrawal>>,
}

/// An error encountered during [OpBlock] conversion.
#[derive(Error, Debug)]
pub enum OpBlockConversionError {
    /// Invalid genesis hash.
    #[error("Invalid genesis hash. Expected {0}, got {1}")]
    InvalidGenesisHash(B256, B256),
    /// Invalid transaction type.
    #[error("First payload transaction has unexpected type: {0}")]
    InvalidTxType(u8),
    /// L1 Info error
    #[error(transparent)]
    L1InfoError(#[from] DecodeError),
    /// Missing system config in genesis block.
    #[error("Missing system config in genesis block")]
    MissingSystemConfigGenesis,
    /// Empty transactions.
    #[error("Empty transactions in payload. Block hash: {0}")]
    EmptyTransactions(B256),
}

/// OP Stack full block.
///
/// Withdrawals can be optionally included at the end of the RLP encoded message.
///
/// Taken from [reth-primitives](https://github.com/paradigmxyz/reth)
#[derive(Debug, Clone, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
#[rlp(trailing)]
pub struct OpBlock {
    /// Block header.
    pub header: Header,
    /// Transactions in this block.
    pub body: Vec<OpTxEnvelope>,
    /// Ommers/uncles header.
    pub ommers: Vec<Header>,
    /// Block withdrawals.
    pub withdrawals: Option<Vec<Withdrawal>>,
}

impl OpBlock {
    /// Converts the [OpBlock] to an [L2BlockInfo], by checking against the L1
    /// information transaction or the genesis block.
    pub fn to_l2_block_ref(
        &self,
        rollup_config: &RollupConfig,
    ) -> Result<L2BlockInfo, OpBlockConversionError> {
        let (l1_origin, sequence_number) = if self.header.number == rollup_config.genesis.l2.number
        {
            if self.header.hash_slow() != rollup_config.genesis.l2.hash {
                return Err(OpBlockConversionError::InvalidGenesisHash(
                    rollup_config.genesis.l2.hash,
                    self.header.hash_slow(),
                ));
            }
            (rollup_config.genesis.l1, 0)
        } else {
            if self.body.is_empty() {
                return Err(OpBlockConversionError::EmptyTransactions(self.header.hash_slow()));
            }

            let OpTxEnvelope::Deposit(ref tx) = self.body[0] else {
                return Err(OpBlockConversionError::InvalidTxType(self.body[0].tx_type() as u8));
            };

            let l1_info = L1BlockInfoTx::decode_calldata(tx.input.as_ref())?;
            (l1_info.id(), l1_info.sequence_number())
        };

        Ok(L2BlockInfo {
            block_info: BlockInfo {
                hash: self.header.hash_slow(),
                number: self.header.number,
                parent_hash: self.header.parent_hash,
                timestamp: self.header.timestamp,
            },
            l1_origin,
            seq_num: sequence_number,
        })
    }

    /// Converts the [OpBlock] to a partial [SystemConfig].
    pub fn to_system_config(
        &self,
        rollup_config: &RollupConfig,
    ) -> Result<SystemConfig, OpBlockConversionError> {
        if self.header.number == rollup_config.genesis.l2.number {
            if self.header.hash_slow() != rollup_config.genesis.l2.hash {
                return Err(OpBlockConversionError::InvalidGenesisHash(
                    rollup_config.genesis.l2.hash,
                    self.header.hash_slow(),
                ));
            }
            return rollup_config
                .genesis
                .system_config
                .ok_or(OpBlockConversionError::MissingSystemConfigGenesis);
        }

        if self.body.is_empty() {
            return Err(OpBlockConversionError::EmptyTransactions(self.header.hash_slow()));
        }
        let OpTxEnvelope::Deposit(ref tx) = self.body[0] else {
            return Err(OpBlockConversionError::InvalidTxType(self.body[0].tx_type() as u8));
        };

        let l1_info = L1BlockInfoTx::decode_calldata(tx.input.as_ref())?;
        let l1_fee_scalar = match l1_info {
            L1BlockInfoTx::Bedrock(L1BlockInfoBedrock { l1_fee_scalar, .. }) => l1_fee_scalar,
            L1BlockInfoTx::Ecotone(L1BlockInfoEcotone {
                base_fee_scalar,
                blob_base_fee_scalar,
                ..
            }) => {
                // Translate Ecotone values back into encoded scalar if needed.
                // We do not know if it was derived from a v0 or v1 scalar,
                // but v1 is fine, a 0 blob base fee has the same effect.
                let mut buf = B256::ZERO;
                buf[0] = 0x01;
                buf[24..28].copy_from_slice(blob_base_fee_scalar.to_be_bytes().as_ref());
                buf[28..32].copy_from_slice(base_fee_scalar.to_be_bytes().as_ref());
                buf.into()
            }
        };

        Ok(SystemConfig {
            batcher_address: l1_info.batcher_address(),
            overhead: l1_info.l1_fee_overhead(),
            scalar: l1_fee_scalar,
            gas_limit: self.header.gas_limit as u64,
            base_fee_scalar: None,
            blob_base_fee_scalar: None,
        })
    }
}
