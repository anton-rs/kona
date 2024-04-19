//! Contains the execution payload type.

use alloc::vec::Vec;
use alloy_primitives::{Address, Bloom, Bytes, B256, U256};
use anyhow::Result;
use op_alloy_consensus::TxDeposit;

/// Fixed and variable memory costs for a payload.
/// ~1000 bytes per payload, with some margin for overhead like map data.
pub const PAYLOAD_MEM_FIXED_COST: u64 = 1000;

/// Memory overhead per payload transaction.
/// 24 bytes per tx overhead (size of slice header in memory).
pub const PAYLOAD_TX_MEM_OVERHEAD: u64 = 24;

use crate::types::{L1BlockInfoBedrock, L1BlockInfoEcotone};

use super::{
    Block, BlockInfo, L1BlockInfoTx, L2BlockInfo, OpBlock, RollupConfig, SystemConfig, Withdrawal,
};
use alloy_rlp::{Decodable, Encodable};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Envelope wrapping the [L2ExecutionPayload].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L2ExecutionPayloadEnvelope {
    /// Parent beacon block root.
    #[cfg_attr(feature = "serde", serde(rename = "parentBeaconBlockRoot"))]
    pub parent_beacon_block_root: Option<B256>,
    /// The inner execution payload.
    #[cfg_attr(feature = "serde", serde(rename = "executionPayload"))]
    pub execution_payload: L2ExecutionPayload,
}

impl L2ExecutionPayloadEnvelope {
    /// Returns the payload memory size.
    pub fn mem_size(&self) -> u64 {
        let mut out = PAYLOAD_MEM_FIXED_COST;
        for tx in &self.execution_payload.transactions {
            out += tx.len() as u64 + PAYLOAD_TX_MEM_OVERHEAD;
        }
        out
    }
}

/// The execution payload.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct L2ExecutionPayload {
    /// The parent hash.
    #[cfg_attr(feature = "serde", serde(rename = "parentHash"))]
    pub parent_hash: B256,
    /// The coinbase address.
    #[cfg_attr(feature = "serde", serde(rename = "feeRecipient"))]
    pub fee_recipient: Address,
    /// The state root.
    #[cfg_attr(feature = "serde", serde(rename = "stateRoot"))]
    pub state_root: B256,
    /// The transactions root.
    #[cfg_attr(feature = "serde", serde(rename = "receiptsRoot"))]
    pub receipts_root: B256,
    /// The logs bloom.
    #[cfg_attr(feature = "serde", serde(rename = "logsBloom"))]
    pub logs_bloom: Bloom,
    /// The mix hash.
    #[cfg_attr(feature = "serde", serde(rename = "prevRandao"))]
    pub prev_randao: B256,
    /// The difficulty.
    #[cfg_attr(feature = "serde", serde(rename = "blockNumber"))]
    pub block_number: u64,
    /// The gas limit.
    #[cfg_attr(feature = "serde", serde(rename = "gasLimit"))]
    pub gas_limit: u64,
    /// The gas used.
    #[cfg_attr(feature = "serde", serde(rename = "gasUsed"))]
    pub gas_used: u64,
    /// The timestamp.
    #[cfg_attr(feature = "serde", serde(rename = "timestamp"))]
    pub timestamp: u64,
    /// The extra data.
    #[cfg_attr(feature = "serde", serde(rename = "extraData"))]
    pub extra_data: Bytes,
    /// Base fee per gas.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "baseFeePerGas", skip_serializing_if = "Option::is_none")
    )]
    pub base_fee_per_gas: Option<u64>,
    /// Block hash.
    #[cfg_attr(feature = "serde", serde(rename = "blockHash"))]
    pub block_hash: B256,
    /// The transactions.
    #[cfg_attr(feature = "serde", serde(rename = "transactions"))]
    pub transactions: Vec<Bytes>,
    /// The withdrawals.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "withdrawals", skip_serializing_if = "Option::is_none")
    )]
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// The  blob gas used.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "blobGasUsed", skip_serializing_if = "Option::is_none")
    )]
    pub blob_gas_used: Option<u64>,
    /// The excess blob gas.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "excessBlobGas", skip_serializing_if = "Option::is_none")
    )]
    pub excess_blob_gas: Option<u64>,
}

impl L2ExecutionPayloadEnvelope {
    /// Converts the [L2ExecutionPayloadEnvelope] to an [L2BlockInfo], by checking against the L1
    /// information transaction or the genesis block.
    pub fn to_l2_block_ref(&self, rollup_config: &RollupConfig) -> Result<L2BlockInfo> {
        let L2ExecutionPayloadEnvelope { execution_payload, .. } = self;

        let (l1_origin, sequence_number) =
            if execution_payload.block_number == rollup_config.genesis.l2.number {
                if execution_payload.block_hash != rollup_config.genesis.l2.hash {
                    anyhow::bail!("Invalid genesis hash");
                }
                (rollup_config.genesis.l1, 0)
            } else {
                if execution_payload.transactions.is_empty() {
                    anyhow::bail!(
                        "L2 block is missing L1 info deposit transaction, block hash: {}",
                        execution_payload.block_hash
                    );
                }

                let tx = TxDeposit::decode(&mut execution_payload.transactions[0][1..].as_ref())
                    .map_err(|e| anyhow::anyhow!(e))?;

                // let OpTxEnvelope::Deposit(tx) = tx else {
                //     anyhow::bail!("First payload transaction has unexpected type: {:?}",
                // tx.tx_type()); };

                let l1_info = L1BlockInfoTx::decode_calldata(tx.input.as_ref())?;
                (l1_info.id(), l1_info.sequence_number())
            };

        Ok(L2BlockInfo {
            block_info: BlockInfo {
                hash: execution_payload.block_hash,
                number: execution_payload.block_number,
                parent_hash: execution_payload.parent_hash,
                timestamp: execution_payload.timestamp,
            },
            l1_origin,
            seq_num: sequence_number,
        })
    }

    /// Converts the [L2ExecutionPayloadEnvelope] to a partial [SystemConfig].
    pub fn to_system_config(&self, rollup_config: &RollupConfig) -> Result<SystemConfig> {
        let L2ExecutionPayloadEnvelope { execution_payload, .. } = self;

        if execution_payload.block_number == rollup_config.genesis.l2.number {
            if execution_payload.block_hash != rollup_config.genesis.l2.hash {
                anyhow::bail!("Invalid genesis hash");
            }
            return Ok(rollup_config.genesis.system_config);
        }

        if execution_payload.transactions.is_empty() {
            anyhow::bail!(
                "L2 block is missing L1 info deposit transaction, block hash: {}",
                execution_payload.block_hash
            );
        }
        let tx = OpTxEnvelope::decode(&mut execution_payload.transactions[0].as_ref())
            .map_err(|e| anyhow::anyhow!(e))?;

        let OpTxEnvelope::Deposit(tx) = tx else {
            anyhow::bail!("First payload transaction has unexpected type: {:?}", tx.tx_type());
        };

        let l1_info = L1BlockInfoTx::decode_calldata(tx.input.as_ref())?;
        let l1_fee_scalar = match l1_info {
            L1BlockInfoTx::Bedrock(L1BlockInfoBedrock { l1_fee_scalar, .. }) => l1_fee_scalar,
            L1BlockInfoTx::Ecotone(L1BlockInfoEcotone {
                blob_base_fee,
                blob_base_fee_scalar,
                ..
            }) => {
                // Translate Ecotone values back into encoded scalar if needed.
                // We do not know if it was derived from a v0 or v1 scalar,
                // but v1 is fine, a 0 blob base fee has the same effect.
                let mut buf = B256::ZERO;
                buf[0] = 0x01;
                buf[24..28].copy_from_slice(blob_base_fee_scalar.to_be_bytes().as_ref());
                buf[28..32].copy_from_slice(blob_base_fee.to_be_bytes().as_ref());
                buf.into()
            }
        };

        Ok(SystemConfig {
            batcher_addr: l1_info.batcher_address(),
            l1_fee_overhead: l1_info.l1_fee_overhead(),
            l1_fee_scalar,
            gas_limit: U256::from(execution_payload.gas_limit),
        })
    }
}

impl From<Block> for L2ExecutionPayloadEnvelope {
    fn from(block: Block) -> Self {
        let Block { header, body, withdrawals, .. } = block;
        Self {
            execution_payload: L2ExecutionPayload {
                parent_hash: header.parent_hash,
                fee_recipient: header.beneficiary,
                state_root: header.state_root,
                receipts_root: header.receipts_root,
                logs_bloom: header.logs_bloom,
                prev_randao: header.difficulty.into(),
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data: header.extra_data.clone(),
                base_fee_per_gas: header.base_fee_per_gas,
                block_hash: header.hash_slow(),
                transactions: body
                    .into_iter()
                    .map(|tx| {
                        let mut buf = Vec::with_capacity(tx.length());
                        tx.encode(&mut buf);
                        buf.into()
                    })
                    .collect(),
                withdrawals,
                blob_gas_used: header.blob_gas_used,
                excess_blob_gas: header.excess_blob_gas,
            },
            parent_beacon_block_root: header.parent_beacon_block_root,
        }
    }
}

impl From<OpBlock> for L2ExecutionPayloadEnvelope {
    fn from(block: OpBlock) -> Self {
        let OpBlock { header, body, withdrawals, .. } = block;
        Self {
            execution_payload: L2ExecutionPayload {
                parent_hash: header.parent_hash,
                fee_recipient: header.beneficiary,
                state_root: header.state_root,
                receipts_root: header.receipts_root,
                logs_bloom: header.logs_bloom,
                prev_randao: header.difficulty.into(),
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data: header.extra_data.clone(),
                base_fee_per_gas: header.base_fee_per_gas,
                block_hash: header.hash_slow(),
                transactions: body
                    .into_iter()
                    .map(|tx| {
                        let mut buf = Vec::with_capacity(tx.length());
                        tx.encode(&mut buf);
                        buf.into()
                    })
                    .collect(),
                withdrawals,
                blob_gas_used: header.blob_gas_used,
                excess_blob_gas: header.excess_blob_gas,
            },
            parent_beacon_block_root: header.parent_beacon_block_root,
        }
    }
}
