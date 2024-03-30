//! This module contains the [SingleBatch] type.

use alloc::vec::Vec;
use alloy_primitives::BlockHash;
use alloy_rlp::{Decodable, Encodable};

use super::batch_validity::BatchValidity;
use super::block::{BlockInfo, L2BlockRef};
use super::rollup_config::RollupConfig;
use super::RawTransaction;

/// Represents a single batch: a single encoded L2 block
#[derive(Debug, Clone, PartialEq, Eq)]
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
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
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
}

impl SingleBatch {
    /// If any transactions are empty or deposited transaction types.
    pub fn has_invalid_transactions(&self) -> bool {
        self.transactions
            .iter()
            .any(|tx| tx.0.is_empty() || tx.0[0] == 0x7E)
    }

    /// Checks if the batch is valid.
    pub fn check_batch(
        &self,
        cfg: &RollupConfig,
        l1_blocks: &[BlockInfo],
        l2_safe_head: L2BlockRef,
        inclusion_block: &BlockInfo,
    ) -> BatchValidity {
        // Sanity check input consistency
        if l1_blocks.is_empty() {
            // TODO: log a warning: "missing L1 block input, cannot proceed with batch checking"
            return BatchValidity::Undecided;
        }

        let epoch = l1_blocks[0];
        let next_timestamp = l2_safe_head.info.timestamp + cfg.block_time;
        if self.timestamp > next_timestamp {
            // TODO: trace log: "received out-of-order batch for future processing after next batch"
            return BatchValidity::Future;
        }
        if self.timestamp < next_timestamp {
            // TODO: warn log: "dropping batch with old timestamp", "min_timestamp", next_timestamp
            return BatchValidity::Drop;
        }

        // Dependent on the above timestamp check.
        // If the timestamp is correct, then it must build on top of the safe head.
        if self.parent_hash != l2_safe_head.info.hash {
            // TODO: warn log: "ignoring batch with mismatching parent hash", "current_safe_head", l2_safe_head.info.hash
            return BatchValidity::Drop;
        }

        // Filter out batches that were included too late.
        if self.epoch_num + cfg.seq_window_size < inclusion_block.number {
            // TODO: warn log: "batch was included too late, sequence window expired"
            return BatchValidity::Drop;
        }

        // Check the L1 origin of the batch
        let mut batch_origin = epoch;
        if self.epoch_num < epoch.number {
            // TODO: warn log: "dropped batch, epoch is too old", "minimum", epoch.id()
            return BatchValidity::Drop;
        } else if self.epoch_num == epoch.number {
            // Batch is sticking to the current epoch, continue.
        } else if self.epoch_num == epoch.number + 1 {
            // With only 1 l1Block we cannot look at the next L1 Origin.
            // Note: This means that we are unable to determine validity of a batch
            // without more information. In this case we should bail out until we have
            // more information otherwise the eager algorithm may diverge from a non-eager
            // algorithm.
            if l1_blocks.len() < 2 {
                // TODO: info log: "eager batch wants to advance epoch, but could not without more L1 blocks", "current_epoch", epoch.id()
                return BatchValidity::Undecided;
            }
            batch_origin = l1_blocks[1];
        } else {
            // TODO: warn log: "batch is for future epoch too far ahead, while it has the next timestamp, so it must be invalid", "current_epoch", epoch.id()
            return BatchValidity::Drop;
        }

        // Validate the batch epoch hash
        if self.epoch_hash != batch_origin.hash {
            // TODO: warn log: "batch is for different L1 chain, epoch hash does not match", "expected", batch_origin.id()
            return BatchValidity::Drop;
        }

        if self.timestamp < batch_origin.timestamp {
            // TODO: warn log: "batch timestamp is less than L1 origin timestamp", "l2_timestamp", self.timestamp, "l1_timestamp", batch_origin.timestamp, "origin", batch_origin.id()
            return BatchValidity::Drop;
        }

        // Check if we ran out of sequencer time drift
        let max = if let Some(max) = batch_origin.timestamp.checked_add(cfg.max_sequencer_drift) {
            max
        } else {
            // TODO: log that the batch exceeds time drift.
            return BatchValidity::Drop;
        };

        let no_txs = self.transactions.is_empty();
        if self.timestamp > max && !no_txs {
            // If the sequencer is ignoring the time drift rule, then drop the batch and force an empty batch instead,
            // as the sequencer is not allowed to include anything past this point without moving to the next epoch.
            // TODO: warn log: "batch exceeded sequencer time drift, sequencer must adopt new L1 origin to include transactions again", "max_time", max
            return BatchValidity::Drop;
        }
        if self.timestamp > max && no_txs {
            // If the sequencer is co-operating by producing an empty batch,
            // allow the batch if it was the right thing to do to maintain the L2 time >= L1 time invariant.
            // Only check batches that do not advance the epoch, to ensure epoch advancement regardless of time drift is allowed.
            if epoch.number == batch_origin.number {
                if l1_blocks.len() < 2 {
                    // TODO: info log: "without the next L1 origin we cannot determine yet if this empty batch that exceeds the time drift is still valid"
                    return BatchValidity::Undecided;
                }
                let next_origin = l1_blocks[1];
                // Check if the next L1 Origin could have been adopted
                if self.timestamp >= next_origin.timestamp {
                    // TODO: log that the batch exceeded the time drift without adopting the next origin.
                    return BatchValidity::Drop;
                } else {
                    // TODO: log that we are continuing with an empty batch before the late L1 block to preserve the L2 time invariant.
                    // TODO: metrice empty batch continuation
                }
            }
        }

        // We can do this check earlier, but it's a more intensive one, so we do this last.
        // TODO: metrice & allow configurability to measure the time it takes to check the batch.
        for tx in self.transactions.iter() {
            if tx.is_empty() {
                // TODO: warn log: "transaction data must not be empty, but found empty tx", "tx_index", i
                return BatchValidity::Drop;
            }
            if tx.is_deposit() {
                // TODO: warn log: "sequencers may not embed any deposits into batch data, but found tx that has one", "tx_index", i
                return BatchValidity::Drop;
            }
        }

        BatchValidity::Accept
    }
}

#[cfg(test)]
mod tests {
    use super::SingleBatch;
    use crate::types::RawTransaction;
    use alloc::vec;
    use alloy_primitives::B256;
    use alloy_rlp::{BytesMut, Decodable, Encodable};

    #[test]
    fn test_single_batch_rlp_roundtrip() {
        let single_batch = SingleBatch {
            parent_hash: B256::ZERO,
            epoch_num: 0xFF,
            epoch_hash: B256::ZERO,
            timestamp: 0xEE,
            transactions: vec![RawTransaction(vec![0x00])],
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
            transactions: vec![RawTransaction(vec![0x7E])],
        };

        assert!(single_batch.has_invalid_transactions());
    }
}
