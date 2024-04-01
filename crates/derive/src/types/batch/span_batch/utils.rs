//! Utilities for Span Batch Encoding and Decoding.

use alloc::vec::Vec;
use alloy_rlp::{Buf, Header};

use super::{SpanBatchError, SpanDecodingError};

/// Reads transaction data from a reader.
pub(crate) fn read_tx_data(r: &mut &[u8]) -> Result<(Vec<u8>, u8), SpanBatchError> {
    let mut tx_data = Vec::new();
    let first_byte = *r.first().ok_or(SpanBatchError::Decoding(
        SpanDecodingError::InvalidTransactionData,
    ))?;
    let mut tx_type = 0;
    if first_byte <= 0x7F {
        // EIP-2718: Non-legacy tx, so write tx type
        tx_type = first_byte;
        tx_data.push(tx_type);
        r.advance(1);
    }

    // Copy the reader, as we need to read the header to determine if the payload is a list.
    // TODO(clabby): This is horribly inefficient. It'd be nice if we could peek at this rather than forcibly having to
    // advance the buffer passed, should read more into the alloy rlp docs to see if this is possible.
    let r_copy = Vec::from(*r);
    let rlp_header = Header::decode(&mut r_copy.as_slice())
        .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionData))?;

    let tx_payload = if rlp_header.list {
        // Grab the raw RLP for the transaction data from `r`. It was unaffected since we copied it.
        let payload_length_with_header = rlp_header.payload_length + rlp_header.length();
        let payload = r[0..payload_length_with_header].to_vec();
        r.advance(payload_length_with_header);
        Ok(payload)
    } else {
        Err(SpanBatchError::Decoding(
            SpanDecodingError::InvalidTransactionData,
        ))
    }?;
    tx_data.extend_from_slice(&tx_payload);

    Ok((tx_data, tx_type))
}

/// Converts a `v` value to a y parity bit, from the transaaction type.
pub(crate) fn convert_v_to_y_parity(v: u64, tx_type: u64) -> Result<bool, SpanBatchError> {
    match tx_type {
        0 => {
            if v != 27 && v != 28 {
                // EIP-155: v = 2 * chain_id + 35 + yParity
                Ok((v - 35) & 1 == 1)
            } else {
                // Unprotected legacy txs must have v = 27 or 28
                Ok(v - 27 == 1)
            }
        }
        1 | 2 => Ok(v == 1),
        _ => Err(SpanBatchError::Decoding(
            SpanDecodingError::InvalidTransactionType,
        )),
    }
}
