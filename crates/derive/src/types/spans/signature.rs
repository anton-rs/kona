//! Span Batch Signature

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "k256")] {
        /// Span Batch Signature
        pub type SpanBatchSignature = alloy_primitives::Signature;
    } else {
        /// Span Batch Signature
        pub type SpanBatchSignature = LocalSpanBatchSignature;
    }
}

#[cfg(not(feature = "k256"))]
use alloy_primitives::{U256, U64};

/// Local Span Batch Signature
#[cfg(not(feature = "k256"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSpanBatchSignature {
    /// The signature r value
    pub r: U256,
    /// The signature s value
    pub s: U256,
    /// The signature v value
    pub v: U64,
}

#[cfg(not(feature = "k256"))]
impl LocalSpanBatchSignature {
    /// Creates a new span batch signature.
    pub fn new(r: U256, s: U256, v: U64) -> Self {
        Self { r, s, v }
    }
}
