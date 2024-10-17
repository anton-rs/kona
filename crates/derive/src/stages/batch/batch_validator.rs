//! Contains the [BatchValidator] stage.

use super::NextBatchProvider;
use crate::{
    batch::{Batch, BatchValidity, SingleBatch},
    errors::ResetError,
    pipeline::{OriginAdvancer, PipelineResult, Signal, SignalReceiver},
    prelude::{OriginProvider, PipelineError, PipelineErrorKind},
    stages::AttributesProvider,
    traits::ResetSignal,
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use tracing::{debug, error, info, warn};

/// The [BatchValidator] stage is responsible for sequencing the [Batch]es for the
/// [AttributesQueue]'s consumption.
///
/// [Batch]: crate::batch::Batch
/// [AttributesQueue]: crate::stages::attributes_queue::AttributesQueue
#[derive(Debug)]
pub struct BatchValidator<P>
where
    P: NextBatchProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug,
{
    /// The rollup configuration.
    cfg: Arc<RollupConfig>,
    /// The previous stage of the derivation pipeline.
    prev: P,
    /// The L1 origin of the batch sequencer.
    origin: Option<BlockInfo>,
    /// A consecutive, time-centric window of L1 Blocks.
    /// Every L1 origin of unsafe L2 Blocks must be included in this list.
    /// If every L2 Block corresponding to a single L1 Block becomes safe,
    /// the block is popped from this list.
    /// If new L2 Block's L1 origin is not included in this list, fetch and
    /// push it to the list.
    l1_blocks: Vec<BlockInfo>,
}

impl<P> BatchValidator<P>
where
    P: NextBatchProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug,
{
    /// Create a new [BatchValidator] stage.
    pub fn new(cfg: Arc<RollupConfig>, prev: P) -> Self {
        Self { cfg, prev, origin: None, l1_blocks: Vec::new() }
    }

    /// Returns `true` if the pipeline origin is behind the parent origin.
    fn origin_behind(&self, parent: &L2BlockInfo) -> bool {
        self.prev.origin().map_or(true, |origin| origin.number < parent.l1_origin.number)
    }

    /// Updates the [BatchValidator]'s view of the L1 origin blocks.
    ///
    fn update_origins(&mut self, parent: &L2BlockInfo) -> PipelineResult<()> {
        // NOTE: The origin is used to determine if it's behind.
        // It is the future origin that gets saved into the l1 blocks array.
        // We always update the origin of this stage if it's not the same so
        // after the update code runs, this is consistent.
        let origin_behind = self.origin_behind(parent);

        // Advance the origin if needed.
        // The entire pipeline has the same origin.
        // Batches prior to the l1 origin of the l2 safe head are not accepted.
        if self.origin != self.prev.origin() {
            self.origin = self.prev.origin();
            if !origin_behind {
                let origin = match self.origin.as_ref().ok_or(PipelineError::MissingOrigin.crit()) {
                    Ok(o) => o,
                    Err(e) => {
                        return Err(e);
                    }
                };
                self.l1_blocks.push(*origin);
            } else {
                // This is to handle the special case of startup.
                // At startup, the batch queue is reset and includes the
                // l1 origin. That is the only time where immediately after
                // reset is called, the origin behind is false.
                self.l1_blocks.clear();
            }
            info!(target: "batch-queue", "Advancing batch queue origin: {:?}", self.origin);
        }

        // If the epoch is advanced, update the l1 blocks.
        // Advancing epoch must be done after the pipeline successfully applies the entire span
        // batch to the chain.
        // Because the span batch can be reverted during processing the batch, then we must
        // preserve existing l1 blocks to verify the epochs of the next candidate batch.
        if !self.l1_blocks.is_empty() && parent.l1_origin.number > self.l1_blocks[0].number {
            let is_holocene_active = self.cfg.is_holocene_active(
                self.origin.ok_or(PipelineError::MissingOrigin.crit())?.timestamp,
            );

            for (i, block) in self.l1_blocks.iter().enumerate() {
                if parent.l1_origin.number == block.number {
                    if is_holocene_active && i > 1 {
                        panic!("Origin jump detected post-Holocene");
                    }

                    self.l1_blocks.drain(0..i);
                    info!(target: "batch-sequencer", "Advancing epoch");
                    break;
                }
            }
            // If the origin of the parent block is not included, we must advance the origin.
        }

        Ok(())
    }

    /// Attempts to derive an empty batch, if the sequencing window is expired.
    fn try_derive_empty_batch(&mut self, parent: &L2BlockInfo) -> PipelineResult<SingleBatch> {
        let epoch = self.l1_blocks[0];

        // If the current epoch is too old compared to the L1 block we are at,
        // i.e. if the sequence window expired, we create empty batches for the current epoch
        let stage_origin = self.origin.ok_or(PipelineError::MissingOrigin.crit())?;
        let expiry_epoch = epoch.number + self.cfg.seq_window_size;
        let force_empty_batches = expiry_epoch <= stage_origin.number;
        let first_of_epoch = epoch.number == parent.l1_origin.number + 1;
        let next_timestamp = epoch.timestamp + self.cfg.block_time;

        // If the sequencer window did not expire,
        // there is still room to receive batches for the current epoch.
        // No need to force-create empty batch(es) towards the next epoch yet.
        if !force_empty_batches {
            return Err(PipelineError::Eof.temp());
        }

        info!(
            target: "batch-sequencer",
            "Generating empty batches for epoch: {} | parent: {}",
            epoch.number, parent.l1_origin.number
        );

        // The next L1 block is needed to proceed towards the next epoch.
        if self.l1_blocks.len() < 2 {
            return Err(PipelineError::Eof.temp());
        }

        let next_epoch = self.l1_blocks[1];

        // Fill with empty L2 blocks of the same epoch until we meet the time of the next L1 origin,
        // to preserve that L2 time >= L1 time. If this is the first block of the epoch, always
        // generate a batch to ensure that we at least have one batch per epoch.
        if next_timestamp < next_epoch.timestamp || first_of_epoch {
            info!(target: "batch-sequencer", "Generating empty batch for epoch: {}", epoch.number);
            return Ok(SingleBatch {
                parent_hash: parent.block_info.hash,
                epoch_num: epoch.number,
                epoch_hash: epoch.hash,
                timestamp: next_timestamp,
                transactions: Vec::new(),
            });
        }

        // At this point we have auto generated every batch for the current epoch
        // that we can, so we can advance to the next epoch.
        info!(
            target: "batch-sequencer",
            "Advancing to next epoch: {}, timestamp: {}, epoch timestamp: {}",
            next_epoch.number, next_timestamp, next_epoch.timestamp
        );
        self.l1_blocks.remove(0);
        Err(PipelineError::Eof.temp())
    }
}

#[async_trait]
impl<P> AttributesProvider for BatchValidator<P>
where
    P: NextBatchProvider + OriginAdvancer + OriginProvider + SignalReceiver + Send + Debug,
{
    async fn next_batch(&mut self, parent: L2BlockInfo) -> PipelineResult<SingleBatch> {
        // Update the L1 origin blocks within the stage.
        self.update_origins(&parent)?;

        // If the origin is behind, we must drain previous stages to catch up.
        let stage_origin = self.origin.ok_or(PipelineError::MissingOrigin.crit())?;
        if self.origin_behind(&parent) || parent.l1_origin.number < stage_origin.number {
            self.prev.next_batch(parent, self.l1_blocks.as_ref()).await?;
            return Err(PipelineError::NotEnoughData.temp());
        }

        // At least the L1 origin of the safe block and the L1 origin of the following block must
        // be included in the l1 blocks.
        if self.l1_blocks.len() < 2 {
            return Err(PipelineError::MissingOrigin.crit());
        }

        // Note: epoch origin can now be one block ahead of the L2 Safe Head
        // This is in the case where we auto generate all batches in an epoch & advance the epoch
        // but don't advance the L2 Safe Head's epoch
        let epoch = self.l1_blocks[0];
        if parent.l1_origin != epoch.id() && parent.l1_origin.number != epoch.number - 1 {
            return Err(PipelineErrorKind::Reset(ResetError::L1OriginMismatch(
                parent.l1_origin.number,
                epoch.number - 1,
            )));
        }

        // Pull the next batch from the previous stage.
        let next_batch = match self.prev.next_batch(parent, self.l1_blocks.as_ref()).await {
            Ok(batch) => batch,
            Err(PipelineErrorKind::Temporary(PipelineError::Eof)) => {
                return self.try_derive_empty_batch(&parent);
            }
            Err(e) => {
                return Err(e);
            }
        };

        // The batch must be a single batch - this stage does not support span batches.
        let Batch::Single(next_batch) = next_batch else {
            error!(
                target: "batch-sequencer",
                "BatchValidator received a batch that is not a SingleBatch"
            );
            return Err(PipelineError::InvalidBatchType.crit());
        };

        // Check the validity of the single batch before forwarding it.
        match next_batch.check_batch(
            self.cfg.as_ref(),
            self.l1_blocks.as_ref(),
            parent,
            &stage_origin,
        ) {
            BatchValidity::Accept => {
                debug!(target: "batch-sequencer", "Found next singular batch");
                Ok(next_batch)
            }
            BatchValidity::Past => {
                warn!(target: "batch-sequencer", "Dropping old batch");
                Err(PipelineError::NotEnoughData.temp())
            }
            BatchValidity::Drop => {
                warn!(target: "batch-sequencer", "Invalid singular batch, flushing current channel.");
                self.prev.flush();
                Err(PipelineError::NotEnoughData.temp())
            }
            BatchValidity::Undecided => Err(PipelineError::NotEnoughData.temp()),
            BatchValidity::Future => {
                error!(target: "batch-sequencer", "Future batch detected in BatchValidator.");
                Err(PipelineError::InvalidBatchValidity.crit())
            }
        }
    }

    fn is_last_in_span(&self) -> bool {
        self.prev.span_buffer_size() == 0
    }
}

impl<P> OriginProvider for BatchValidator<P>
where
    P: NextBatchProvider + OriginAdvancer + OriginProvider + SignalReceiver + Debug,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P> SignalReceiver for BatchValidator<P>
where
    P: NextBatchProvider + OriginAdvancer + OriginProvider + SignalReceiver + Send + Debug,
{
    async fn signal(&mut self, signal: Signal) -> PipelineResult<()> {
        match signal {
            s @ Signal::Reset(ResetSignal { l1_origin, .. }) => {
                self.prev.signal(s).await?;
                self.origin = Some(l1_origin);
                // Include the new origin as an origin to build on.
                // This is only for the initialization case.
                // During normal resets we will later throw out this block.
                self.l1_blocks.clear();
                self.l1_blocks.push(l1_origin);
                crate::inc!(STAGE_RESETS, &["batch-sequencer"]);
            }
            s @ Signal::Activation(_) | s @ Signal::FlushChannel => {
                self.prev.signal(s).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::BatchValidator;
    use crate::{
        batch::{Batch, SingleBatch},
        pipeline::PipelineResult,
        prelude::PipelineError,
        stages::AttributesProvider,
        test_utils::TestBatchQueueProvider,
    };
    use alloc::sync::Arc;
    use alloy_eips::BlockNumHash;
    use alloy_primitives::B256;
    use op_alloy_genesis::RollupConfig;
    use op_alloy_protocol::{BlockInfo, L2BlockInfo};

    #[tokio::test]
    async fn test_batch_validator_origin_behind_eof() {
        let cfg = Arc::new(RollupConfig::default());
        let mut mock = TestBatchQueueProvider::new(vec![]);
        mock.origin = Some(BlockInfo::default());
        let mut bv = BatchValidator::new(cfg, mock);
        bv.origin = Some(BlockInfo::default());

        let mock_parent = L2BlockInfo {
            l1_origin: BlockNumHash { number: 5, ..Default::default() },
            ..Default::default()
        };
        assert_eq!(bv.next_batch(mock_parent).await.unwrap_err(), PipelineError::Eof.temp());
    }

    #[tokio::test]
    async fn test_batch_validator_origin_behind_out_of_data() {
        let cfg = Arc::new(RollupConfig::default());
        let mut mock = TestBatchQueueProvider::new(
            (0..5).into_iter().map(|_| Ok(Batch::Single(SingleBatch::default()))).collect(),
        );
        mock.origin = Some(BlockInfo::default());
        let mut bv = BatchValidator::new(cfg, mock);
        bv.origin = Some(BlockInfo::default());

        let mock_parent = L2BlockInfo {
            l1_origin: BlockNumHash { number: 5, ..Default::default() },
            ..Default::default()
        };
        assert_eq!(
            bv.next_batch(mock_parent).await.unwrap_err(),
            PipelineError::NotEnoughData.temp()
        );
    }

    #[tokio::test]
    async fn test_batch_validator_next_batch_valid() {
        let cfg = Arc::new(RollupConfig {
            max_sequencer_drift: 700,
            holocene_time: Some(0),
            ..Default::default()
        });
        assert!(cfg.is_holocene_active(0));
        let batch = SingleBatch {
            parent_hash: B256::default(),
            epoch_num: 0,
            epoch_hash: B256::default(),
            timestamp: 100,
            transactions: Vec::new(),
        };
        let parent = L2BlockInfo {
            block_info: BlockInfo { timestamp: 100, ..Default::default() },
            ..Default::default()
        };

        // Setup batch validator deps
        let batch_vec = vec![PipelineResult::Ok(Batch::Single(batch.clone()))];
        let mut mock = TestBatchQueueProvider::new(batch_vec);
        mock.origin = Some(BlockInfo::default());

        // Configure batch validator
        let mut bv = BatchValidator::new(cfg, mock);
        bv.origin = Some(BlockInfo::default());
        bv.l1_blocks.push(BlockInfo::default());
        bv.l1_blocks.push(BlockInfo::default());

        // Grab the next batch.
        assert!(bv.next_batch(parent).await.is_ok());
    }
}
