//! Span Batch Element

#![allow(unused)]

use super::SpanBatchTransactions;
use crate::types::SingleBatch;
use alloc::vec::Vec;

/// A single batch element is similar to the [SingleBatch] type
/// but does not contain the parent hash and epoch hash since spans
/// do not contain this data for every block in the span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanBatchElement {
    /// The epoch number of the L1 block
    pub epoch_num: u64,
    /// The timestamp of the L2 block
    pub timestamp: u64,
    /// The transactions in the L2 block
    pub transactions: Vec<Vec<u8>>,
}

// impl From<SingleBatch> for SpanBatchElement {
//     fn from(batch: SingleBatch) -> Self {
//         SpanBatchElement {
//             epoch_num: batch.epoch_num,
//             timestamp: batch.timestamp,
//             transactions: batch.transactions,
//         }
//     }
// }
