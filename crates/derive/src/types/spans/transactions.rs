//! Span Batch Transactions

use alloy_primitives::U64;
use alloy_rlp::Decodable;

/// Transactions in a span batch
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanBatchTransactions {
    /// The total block transaction count
    pub total_block_tx_count: U64,

    // TODO(refcell): Add in the rest of the fields
    // https://github.com/ethereum-optimism/optimism/blob/develop/op-node/rollup/derive/span_batch_txs.go#L17
}

impl Decodable for SpanBatchTransactions {
    fn decode(r: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let mut transactions = SpanBatchTransactions::default();
        // transactions
        //     .decode_total_block_tx_count(r)
        //     .map_err(|_| alloy_rlp::Error::Custom("Decoding total block tx count failed"))?;
        Ok(transactions)
    }
}
