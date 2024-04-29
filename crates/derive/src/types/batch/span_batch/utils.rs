//! Utilities for Span Batch Encoding and Decoding.

use super::{SpanBatchError, SpanDecodingError};
use alloc::vec::Vec;
use alloy_consensus::{TxEnvelope, TxType};
use alloy_rlp::{Buf, Header};

/// Reads transaction data from a reader.
pub(crate) fn read_tx_data(r: &mut &[u8]) -> Result<(Vec<u8>, TxType), SpanBatchError> {
    let mut tx_data = Vec::new();
    let first_byte =
        *r.first().ok_or(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;
    let mut tx_type = 0;
    if first_byte <= 0x7F {
        // EIP-2718: Non-legacy tx, so write tx type
        tx_type = first_byte;
        tx_data.push(tx_type);
        r.advance(1);
    }

    // Read the RLP header with a different reader pointer. This prevents the initial pointer from
    // being advanced in the case that what we read is invalid.
    let rlp_header = Header::decode(&mut (**r).as_ref())
        .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;

    let tx_payload = if rlp_header.list {
        // Grab the raw RLP for the transaction data from `r`. It was unaffected since we copied it.
        let payload_length_with_header = rlp_header.payload_length + rlp_header.length();
        let payload = r[0..payload_length_with_header].to_vec();
        r.advance(payload_length_with_header);
        Ok(payload)
    } else {
        Err(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))
    }?;
    tx_data.extend_from_slice(&tx_payload);

    Ok((
        tx_data,
        tx_type
            .try_into()
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionType))?,
    ))
}

/// Converts a `v` value to a y parity bit, from the transaaction type.
pub(crate) fn convert_v_to_y_parity(v: u64, tx_type: TxType) -> Result<bool, SpanBatchError> {
    match tx_type {
        TxType::Legacy => {
            if v != 27 && v != 28 {
                // EIP-155: v = 2 * chain_id + 35 + yParity
                Ok((v - 35) & 1 == 1)
            } else {
                // Unprotected legacy txs must have v = 27 or 28
                Ok(v - 27 == 1)
            }
        }
        TxType::Eip2930 | TxType::Eip1559 => Ok(v == 1),
        _ => Err(SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionType)),
    }
}

/// Checks if the signature of the passed [TxEnvelope] is protected.
pub(crate) fn is_protected_v(tx: &TxEnvelope) -> bool {
    match tx {
        TxEnvelope::Legacy(tx) => {
            let v = tx.signature().v().to_u64();
            if 64 - v.leading_zeros() <= 8 {
                return v != 27 && v != 28 && v != 1 && v != 0;
            }
            // anything not 27 or 28 is considered protected
            true
        }
        _ => true,
    }
}
