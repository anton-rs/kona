//! This module contains the [SpanBatchSignature] type, which represents the ECDSA signature of a
//! transaction within a span batch.

use super::{convert_v_to_y_parity, SpanBatchError, SpanDecodingError};
use alloy_consensus::TxType;
use alloy_primitives::{Signature, U256};

/// The ECDSA signature of a transaction within a span batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanBatchSignature {
    pub(crate) v: u64,
    pub(crate) r: U256,
    pub(crate) s: U256,
}

impl From<Signature> for SpanBatchSignature {
    fn from(value: Signature) -> Self {
        Self { v: value.v().to_u64(), r: value.r(), s: value.s() }
    }
}

impl TryFrom<SpanBatchSignature> for Signature {
    type Error = SpanBatchError;

    fn try_from(value: SpanBatchSignature) -> Result<Self, Self::Error> {
        Self::from_rs_and_parity(value.r, value.s, convert_v_to_y_parity(value.v, TxType::Legacy)?)
            .map_err(|_| SpanBatchError::Decoding(SpanDecodingError::InvalidTransactionSignature))
    }
}
