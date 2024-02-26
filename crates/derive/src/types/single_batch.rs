use super::RawTransaction;
use alloc::vec::Vec;
use alloy_primitives::BlockHash;
use alloy_rlp::Decodable;

/// Represents a single batch: a single encoded L2 block
#[derive(Debug, Clone)]
pub struct SingleBatch {
    /// Block hash of the previous L2 block
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
    /// Decodes RLP bytes into a [SingleBatch]
    pub fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let parent_hash = Decodable::decode(buf)?;
        let epoch_num = Decodable::decode(buf)?;
        let epoch_hash = Decodable::decode(buf)?;
        let timestamp = Decodable::decode(buf)?;
        let transactions = Decodable::decode(buf)?;

        Ok(SingleBatch {
            parent_hash,
            epoch_num,
            epoch_hash,
            timestamp,
            transactions,
        })
    }

    /// If any transactions are empty or deposited transaction types.
    pub fn has_invalid_transactions(&self) -> bool {
        self.transactions
            .iter()
            .any(|tx| tx.0.is_empty() || tx.0[0] == 0x7E)
    }
}
