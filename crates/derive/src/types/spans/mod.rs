//! Contains all Span Batch Logic
//!
//! ### Span Batch format
//!
//! Span Batch Type: 1
//! Span Batch: Span Batch Type ++ Prefix ++ Payload
//! Prefix: rel_timestamp ++ l1_origin_num ++ parent_check ++ l1_origin_check
//! Payload: block_count ++ origin_bits ++ block_tx_counts ++ txs
//! Transactions: contract_creation_bits ++ y_parity_bits ++ tx_sigs ++ tx_tos ++ tx_datas ++ tx_nonces ++ tx_gases ++ protected_bits

/// The span batch type
pub const SPAN_BATCH_TYPE: u8 = 0x01;

/// The maximum amount of bytes that will be needed to decode every span
/// batch field. This value cannot be larger than [MAX_RLP_BYTES_PER_CHANNEL]
/// because single batch cannot be larger than channel size.
pub const MAX_SPAN_BATCH_SIZE: usize = MAX_RLP_BYTES_PER_CHANNEL;

/// The maximum amount of bytes that will be read from
/// a channel. This limit is set when decoding the RLP.
pub const MAX_RLP_BYTES_PER_CHANNEL: usize = 10_000_000;

mod bits;
pub use bits::SpanBatchBits;

mod transactions;
pub use transactions::SpanBatchTransactions;

mod batch;
pub use batch::SpanBatch;

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

mod builder;
pub use builder::SpanBatchBuilder;
