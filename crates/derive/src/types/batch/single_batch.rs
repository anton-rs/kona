//! This module contains the [SingleBatch] type.

use crate::types::RawTransaction;
use alloc::vec::Vec;
use alloy_primitives::BlockHash;
use alloy_rlp::{Decodable, Encodable};

/// Represents a single batch: a single encoded L2 block
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SingleBatch {
    /// Block hash of the previous L2 block. `B256::ZERO` if it has not been set by the Batch Queue.
    pub parent_hash: BlockHash,
    /// The batch epoch number. Same as the first L1 block number in the epoch.
    pub epoch_num: u64,
    /// The block hash of the first L1 block in the epoch
    pub epoch_hash: BlockHash,
    /// The L2 block timestamp of this batch
    pub timestamp: u64,
    /// The L2 block transactions in this batch
    pub transactions: Vec<RawTransaction>,
}

impl SingleBatch {
    /// If any transactions are empty or deposited transaction types.
    pub fn has_invalid_transactions(&self) -> bool {
        self.transactions
            .iter()
            .any(|tx| tx.0.is_empty() || tx.0[0] == 0x7E)
    }
}

impl Encodable for SingleBatch {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.parent_hash.encode(out);
        self.epoch_num.encode(out);
        self.epoch_hash.encode(out);
        self.timestamp.encode(out);
        self.transactions.encode(out);
    }
}

impl Decodable for SingleBatch {
    fn decode(rlp: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let parent_hash = BlockHash::decode(rlp)?;
        let epoch_num = u64::decode(rlp)?;
        let epoch_hash = BlockHash::decode(rlp)?;
        let timestamp = u64::decode(rlp)?;
        let transactions = Vec::<RawTransaction>::decode(rlp)?;
        Ok(Self {
            parent_hash,
            epoch_num,
            epoch_hash,
            timestamp,
            transactions,
        })
    }
}

#[cfg(test)]
mod test {
    use super::SingleBatch;
    use crate::types::RawTransaction;
    use alloc::vec;
    use alloy_primitives::{hex, B256};
    use alloy_rlp::{BytesMut, Decodable, Encodable};

    #[test]
    fn test_single_batch_rlp_roundtrip() {
        let single_batch = SingleBatch {
            parent_hash: B256::ZERO,
            epoch_num: 0xFF,
            epoch_hash: B256::ZERO,
            timestamp: 0xEE,
            transactions: vec![RawTransaction(hex!("00").into())],
        };

        let mut out_buf = BytesMut::default();
        single_batch.encode(&mut out_buf);
        let decoded = SingleBatch::decode(&mut out_buf.as_ref()).unwrap();
        assert_eq!(decoded, single_batch);
        assert!(!single_batch.has_invalid_transactions());
    }

    #[test]
    fn test_single_batch_invalid_transactions() {
        let single_batch = SingleBatch {
            parent_hash: B256::ZERO,
            epoch_num: 0xFF,
            epoch_hash: B256::ZERO,
            timestamp: 0xEE,
            transactions: vec![RawTransaction(hex!("7E").into())],
        };

        assert!(single_batch.has_invalid_transactions());
    }
}
