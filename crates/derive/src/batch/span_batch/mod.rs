//! Contains all Span Batch types and logic.
//!
//! ## Batch format
//!
//! ```text
//! [SPAN_BATCH_TYPE] = 1
//! span_batch = [SPAN_BATCH_TYPE] ++ prefix ++ payload
//! prefix = rel_timestamp ++ l1_origin_num ++ parent_check ++ l1_origin_check
//! payload = block_count ++ origin_bits ++ block_tx_counts ++ txs
//! txs = contract_creation_bits ++ y_parity_bits ++ tx_sigs ++ tx_tos ++ tx_datas ++ tx_nonces ++ tx_gases ++ protected_bits
//! ```

/// MAX_SPAN_BATCH_ELEMENTS is the maximum number of blocks, transactions in total,
/// or transaction per block allowed in a span batch.
pub const MAX_SPAN_BATCH_ELEMENTS: u64 = 10_000_000;

mod batch;
pub use batch::SpanBatch;

mod bits;
pub use bits::SpanBatchBits;

mod payload;
pub use payload::SpanBatchPayload;

mod prefix;
pub use prefix::SpanBatchPrefix;

mod errors;
pub use errors::*; // Re-export all error types

mod raw;
pub use raw::RawSpanBatch;

mod element;
pub use element::SpanBatchElement;

mod signature;
pub use signature::SpanBatchSignature;

mod tx_data;
pub use tx_data::{
    SpanBatchEip1559TransactionData, SpanBatchEip2930TransactionData,
    SpanBatchLegacyTransactionData, SpanBatchTransactionData,
};

mod transactions;
pub use transactions::SpanBatchTransactions;

mod utils;
pub(crate) use utils::{convert_v_to_y_parity, read_tx_data};
