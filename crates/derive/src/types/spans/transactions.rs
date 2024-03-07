//! Span Batch Transactions

use crate::types::spans::{SpanBatchBits, SpanBatchSignature};
use alloy_primitives::U64;
use alloy_rlp::Decodable;

/// Transactions in a span batch
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanBatchTransactions {
    /// The total block transaction count
    pub total_block_tx_count: U64,

    // 8 fields
    /// The contract creation bits
    pub contract_creation_bits: SpanBatchBits,
    /// The y parity bits
    pub y_parity_bits: SpanBatchBits,
    /// The transaction signatures
    pub tx_sigs: Vec<SpanBatchSignature>,




	// 8 fields
	contractCreationBits *big.Int // standard span-batch bitlist
	yParityBits          *big.Int // standard span-batch bitlist
	txSigs               []spanBatchSignature
	txNonces             []uint64
	txGases              []uint64
	txTos                []common.Address
	txDatas              []hexutil.Bytes
	protectedBits        *big.Int // standard span-batch bitlist

	// intermediate variables which can be recovered
	txTypes            []int
	totalLegacyTxCount uint64

    // TODO(refcell): Add in the rest of the fields
    // https://github.com/ethereum-optimism/optimism/blob/develop/op-node/rollup/derive/span_batch_txs.go#L17
}

impl Decodable for SpanBatchTransactions {
    fn decode(_r: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let transactions = SpanBatchTransactions::default();
        // transactions
        //     .decode_total_block_tx_count(r)
        //     .map_err(|_| alloy_rlp::Error::Custom("Decoding total block tx count failed"))?;
        Ok(transactions)
    }
}
