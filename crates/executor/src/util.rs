//! Contains utilities for the L2 executor.

use alloc::vec::Vec;
use alloy_consensus::{Eip658Value, Receipt, ReceiptWithBloom};
use alloy_primitives::{Bloom, Log};
use op_alloy_consensus::{
    OpDepositReceipt, OpDepositReceiptWithBloom, OpReceiptEnvelope, OpTxEnvelope, OpTxType,
};

/// Constructs a [OpReceiptEnvelope] from a [Receipt] fields and [OpTxType].
pub(crate) fn receipt_envelope_from_parts<'a>(
    status: bool,
    cumulative_gas_used: u128,
    logs: impl IntoIterator<Item = &'a Log>,
    tx_type: OpTxType,
    deposit_nonce: Option<u64>,
    deposit_receipt_version: Option<u64>,
) -> OpReceiptEnvelope {
    let logs = logs.into_iter().cloned().collect::<Vec<_>>();
    let logs_bloom = logs_bloom(&logs);
    let inner_receipt = Receipt { status: Eip658Value::Eip658(status), cumulative_gas_used, logs };
    match tx_type {
        OpTxType::Legacy => {
            OpReceiptEnvelope::Legacy(ReceiptWithBloom { receipt: inner_receipt, logs_bloom })
        }
        OpTxType::Eip2930 => {
            OpReceiptEnvelope::Eip2930(ReceiptWithBloom { receipt: inner_receipt, logs_bloom })
        }
        OpTxType::Eip1559 => {
            OpReceiptEnvelope::Eip1559(ReceiptWithBloom { receipt: inner_receipt, logs_bloom })
        }
        OpTxType::Deposit => {
            let inner = OpDepositReceiptWithBloom {
                receipt: OpDepositReceipt {
                    inner: inner_receipt,
                    deposit_nonce,
                    deposit_receipt_version,
                },
                logs_bloom,
            };
            OpReceiptEnvelope::Deposit(inner)
        }
    }
}

/// Compute the logs bloom filter for the given logs.
pub(crate) fn logs_bloom<'a>(logs: impl IntoIterator<Item = &'a Log>) -> Bloom {
    let mut bloom = Bloom::ZERO;
    for log in logs {
        bloom.m3_2048(log.address.as_slice());
        for topic in log.topics() {
            bloom.m3_2048(topic.as_slice());
        }
    }
    bloom
}

/// Extract the gas limit from an [OpTxEnvelope].
pub(crate) fn extract_tx_gas_limit(tx: &OpTxEnvelope) -> u128 {
    match tx {
        OpTxEnvelope::Legacy(tx) => tx.tx().gas_limit.into(),
        OpTxEnvelope::Eip2930(tx) => tx.tx().gas_limit.into(),
        OpTxEnvelope::Eip1559(tx) => tx.tx().gas_limit.into(),
        OpTxEnvelope::Deposit(tx) => tx.gas_limit.into(),
        _ => unreachable!(),
    }
}
