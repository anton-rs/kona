//! Error types across derivation stages.

use op_alloy_protocol::MAX_SPAN_BATCH_ELEMENTS;

/// A frame decompression error.
#[derive(derive_more::Display, Debug, PartialEq, Eq)]
pub enum BatchDecompressionError {
    /// The buffer exceeds the [MAX_SPAN_BATCH_ELEMENTS] protocol parameter.
    #[display("The batch exceeds the maximum number of elements: {max_size}", max_size = MAX_SPAN_BATCH_ELEMENTS)]
    BatchTooLarge,
}

impl core::error::Error for BatchDecompressionError {}
