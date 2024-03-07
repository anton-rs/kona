//! Span Batch Signature

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "k256")] {
        use alloy_primitives::Signature;
    } else {
        use crate::types::spans::SpanBatchSignature;
    }
}

/// Span Batch Signature
///
/// The signature of a span batch.
/// If the `k256` feature is enabled, kona will use the [Signature] type from the
/// `alloy_primitives` crate. Otherwise, we need to use our own type.
#[cfg(not(feature = "k256"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatchSignature {
    /// The signature r value
    pub r: [u8; 32],
    /// The signature s value
    pub s: [u8; 32],
    /// The signature v value
    pub v: u8,
}

#[cfg(not(feature = "k256"))]
impl SpanBatchSignature {
    /// Creates a new span batch signature.
    pub fn new(r: [u8; 32], s: [u8; 32], v: u8) -> Self {
        Self { r, s, v }
    }
}
