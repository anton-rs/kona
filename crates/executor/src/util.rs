//! Contains utilities for the L2 executor.

use alloy_consensus::Transaction;
use alloy_primitives::{Bloom, Log};
use op_alloy_consensus::{OpReceiptEnvelope, OpReceiptWithBloom, OpTxEnvelope, OpTxType};

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

/// Wrap an [OpReceiptWithBloom] in an [OpReceiptEnvelope], provided the receipt and a [OpTxType].
pub(crate) fn wrap_receipt_with_bloom<T>(
    receipt: OpReceiptWithBloom<T>,
    tx_type: OpTxType,
) -> OpReceiptEnvelope<T> {
    match tx_type {
        OpTxType::Legacy => OpReceiptEnvelope::Legacy(receipt),
        OpTxType::Eip2930 => OpReceiptEnvelope::Eip2930(receipt),
        OpTxType::Eip1559 => OpReceiptEnvelope::Eip1559(receipt),
        OpTxType::Eip4844 => OpReceiptEnvelope::Eip4844(receipt),
        OpTxType::Deposit => OpReceiptEnvelope::Deposit(receipt),
    }
}

/// Extract the gas limit from an [OpTxEnvelope].
pub(crate) fn extract_tx_gas_limit(tx: &OpTxEnvelope) -> u128 {
    match tx {
        OpTxEnvelope::Legacy(tx) => tx.tx().gas_limit,
        OpTxEnvelope::Eip2930(tx) => tx.tx().gas_limit,
        OpTxEnvelope::Eip1559(tx) => tx.tx().gas_limit,
        OpTxEnvelope::Eip4844(tx) => tx.tx().gas_limit(),
        OpTxEnvelope::Deposit(tx) => tx.gas_limit,
        _ => unreachable!(),
    }
}

/// Returns whether or not an [OpTxEnvelope] is a system transaction.
pub(crate) fn is_system_transaction(tx: &OpTxEnvelope) -> bool {
    match tx {
        OpTxEnvelope::Deposit(tx) => tx.is_system_transaction,
        _ => false,
    }
}
