//! This module contains the `BatchQueue` stage implementation.

use crate::{
    stages::attributes_queue::AttributesProvider,
    traits::{L2ChainProvider, OriginProvider, ResettableStage},
    types::{
        Batch, BatchValidity, BatchWithInclusionBlock, BlockInfo, L2BlockInfo, RollupConfig,
        SingleBatch, StageError, StageResult, SystemConfig,
    },
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use anyhow::anyhow;
use async_trait::async_trait;
use core::fmt::Debug;
use tracing::{error, info, warn};

/// Provides [Batch]es for the [BatchQueue] stage.
#[async_trait]
pub trait BatchQueueProvider {
    /// Returns the next [Batch] in the [ChannelReader] stage, if the stage is not complete.
    /// This function can only be called once while the stage is in progress, and will return
    /// [`None`] on subsequent calls unless the stage is reset or complete. If the stage is
    /// complete and the batch has been consumed, an [StageError::Eof] error is returned.
    ///
    /// [ChannelReader]: crate::stages::ChannelReader
    async fn next_batch(&mut self) -> StageResult<Batch>;
}

/// [BatchQueue] is responsible for o rdering unordered batches
/// and gnerating empty batches when the sequence window has passed.
///
/// It receives batches that are tagged with the L1 Inclusion block of the batch.
/// It only considers batches that are inside the sequencing window of a specific L1 Origin.
/// It tries to eagerly pull batches based on the current L2 safe head.
/// Otherwise it filters/creates an entire epoch's worth of batches at once.
///
/// This stage tracks a range of L1 blocks with the assumption that all batches with an L1 inclusion
/// block inside that range have been added to the stage by the time that it attempts to advance a
/// full epoch.
///
/// It is internally responsible for making sure that batches with L1 inclusions block outside it's
/// working range are not considered or pruned.
#[derive(Debug)]
pub struct BatchQueue<P, BF>
where
    P: BatchQueueProvider + OriginProvider + Debug,
    BF: L2ChainProvider + Debug,
{
    /// The rollup config.
    cfg: Arc<RollupConfig>,
    /// The previous stage of the derivation pipeline.
    prev: P,
    /// The l1 block ref
    origin: Option<BlockInfo>,

    /// A consecutive, time-centric window of L1 Blocks.
    /// Every L1 origin of unsafe L2 Blocks must be included in this list.
    /// If every L2 Block corresponding to a single L1 Block becomes safe,
    /// the block is popped from this list.
    /// If new L2 Block's L1 origin is not included in this list, fetch and
    /// push it to the list.
    l1_blocks: Vec<BlockInfo>,

    /// A set of batches in order from when we've seen them.
    batches: Vec<BatchWithInclusionBlock>,

    /// A set of cached [SingleBatch]es derived from [SpanBatch]es.
    ///
    /// [SpanBatch]: crate::types::SpanBatch
    next_spans: Vec<SingleBatch>,

    /// Used to validate the batches.
    fetcher: BF,
}

impl<P, BF> BatchQueue<P, BF>
where
    P: BatchQueueProvider + OriginProvider + Debug,
    BF: L2ChainProvider + Debug,
{
    /// Creates a new [BatchQueue] stage.
    pub fn new(cfg: Arc<RollupConfig>, prev: P, fetcher: BF) -> Self {
        Self {
            cfg,
            prev,
            origin: None,
            l1_blocks: Vec::new(),
            batches: Vec::new(),
            next_spans: Vec::new(),
            fetcher,
        }
    }

    /// Pops the next batch from the current queued up span-batch cache.
    /// The parent is used to set the parent hash of the batch.
    /// The parent is verified when the batch is later validated.
    pub fn pop_next_batch(&mut self, parent: L2BlockInfo) -> Option<SingleBatch> {
        if self.next_spans.is_empty() {
            panic!("Invalid state: must have next spans to pop");
        }
        let mut next = self.next_spans.remove(0);
        next.parent_hash = parent.block_info.hash;
        Some(next)
    }

    /// Derives the next batch to apply on top of the current L2 safe head.
    /// Follows the validity rules imposed on consecutive batches.
    /// Based on currently available buffered batch and L1 origin information.
    /// A [StageError::Eof] is returned if no batch can be derived yet.
    pub async fn derive_next_batch(
        &mut self,
        empty: bool,
        parent: L2BlockInfo,
    ) -> StageResult<Batch> {
        // Cannot derive a batch if no origin was prepared.
        if self.l1_blocks.is_empty() {
            return Err(StageError::MissingOrigin);
        }

        // Get the epoch
        let epoch = self.l1_blocks[0];
        info!("Deriving next batch for epoch: {}", epoch.number);

        // Note: epoch origin can now be one block ahead of the L2 Safe Head
        // This is in the case where we auto generate all batches in an epoch & advance the epoch
        // but don't advance the L2 Safe Head's epoch
        if parent.l1_origin != epoch.id() && parent.l1_origin.number != epoch.number - 1 {
            return Err(StageError::Custom(anyhow!(
                "buffered L1 chain epoch {} in batch queue does not match safe head origin {:?}",
                epoch,
                parent.l1_origin
            )));
        }

        // Find the first-seen batch that matches all validity conditions.
        // We may not have sufficient information to proceed filtering, and then we stop.
        // There may be none: in that case we force-create an empty batch
        let mut next_batch = None;
        let next_timestamp = parent.block_info.timestamp + self.cfg.block_time;

        // Go over all batches, in order of inclusion, and find the first batch we can accept.
        // Filter in-place by only remembering the batches that may be processed in the future, or
        // any undecided ones.
        let mut remaining = Vec::new();
        for i in 0..self.batches.len() {
            let batch = &self.batches[i];
            let validity =
                batch.check_batch(&self.cfg, &self.l1_blocks, parent, &mut self.fetcher).await;
            match validity {
                BatchValidity::Future => {
                    remaining.push(batch.clone());
                }
                BatchValidity::Drop => {
                    warn!("Dropping batch: {:?}, parent: {}", batch.batch, parent.block_info);
                    continue;
                }
                BatchValidity::Accept => {
                    next_batch = Some(batch.clone());
                    // Don't keep the current batch in the remaining items since we are processing
                    // it now, but retain every batch we didn't get to yet.
                    remaining.extend_from_slice(&self.batches[i + 1..]);
                    break;
                }
                BatchValidity::Undecided => {
                    remaining.extend_from_slice(&self.batches[i..]);
                    self.batches = remaining;
                    return Err(StageError::Eof);
                }
            }
        }
        self.batches = remaining;

        if let Some(nb) = next_batch {
            info!("Next batch found: {:?}", nb.batch);
            return Ok(nb.batch);
        }

        // If the current epoch is too old compared to the L1 block we are at,
        // i.e. if the sequence window expired, we create empty batches for the current epoch
        let expiry_epoch = epoch.number + self.cfg.seq_window_size;
        let force_empty_batches = (expiry_epoch == parent.l1_origin.number && empty) ||
            expiry_epoch < parent.l1_origin.number;
        let first_of_epoch = epoch.number == parent.l1_origin.number + 1;

        // If the sequencer window did not expire,
        // there is still room to receive batches for the current epoch.
        // No need to force-create empty batch(es) towards the next epoch yet.
        if !force_empty_batches {
            return Err(StageError::Eof);
        }

        info!(
            "Generating empty batches for epoch: {} | parent: {}",
            epoch.number, parent.l1_origin.number
        );

        // The next L1 block is needed to proceed towards the next epoch.
        if self.l1_blocks.len() < 2 {
            return Err(StageError::Eof);
        }

        let next_epoch = self.l1_blocks[1];

        // Fill with empty L2 blocks of the same epoch until we meet the time of the next L1 origin,
        // to preserve that L2 time >= L1 time. If this is the first block of the epoch, always
        // generate a batch to ensure that we at least have one batch per epoch.
        if next_timestamp < next_epoch.timestamp || first_of_epoch {
            info!("Generating empty batch for epoch: {}", epoch.number);
            return Ok(Batch::Single(SingleBatch {
                parent_hash: parent.block_info.hash,
                epoch_num: epoch.number,
                epoch_hash: epoch.hash,
                timestamp: next_timestamp,
                transactions: Vec::new(),
            }));
        }

        // At this point we have auto generated every batch for the current epoch
        // that we can, so we can advance to the next epoch.
        info!(
            "Advancing to next epoch: {}, timestamp: {}, epoch timestamp: {}",
            next_epoch.number, next_timestamp, next_epoch.timestamp
        );
        self.l1_blocks.remove(0);
        Err(StageError::Eof)
    }

    /// Adds a batch to the queue.
    pub async fn add_batch(&mut self, batch: Batch, parent: L2BlockInfo) -> StageResult<()> {
        if self.l1_blocks.is_empty() {
            error!("Cannot add batch without an origin");
            panic!("Cannot add batch without an origin");
        }
        let origin = self.origin.ok_or_else(|| anyhow!("cannot add batch with missing origin"))?;
        let data = BatchWithInclusionBlock { inclusion_block: origin, batch };
        // If we drop the batch, validation logs the drop reason with WARN level.
        if data.check_batch(&self.cfg, &self.l1_blocks, parent, &mut self.fetcher).await.is_drop() {
            return Ok(());
        }
        self.batches.push(data);
        Ok(())
    }
}

#[async_trait]
impl<P, BF> AttributesProvider for BatchQueue<P, BF>
where
    P: BatchQueueProvider + OriginProvider + Send + Debug,
    BF: L2ChainProvider + Send + Debug,
{
    /// Returns the next valid batch upon the given safe head.
    /// Also returns the boolean that indicates if the batch is the last block in the batch.
    async fn next_batch(&mut self, parent: L2BlockInfo) -> StageResult<SingleBatch> {
        if !self.next_spans.is_empty() {
            // There are cached singular batches derived from the span batch.
            // Check if the next cached batch matches the given parent block.
            if self.next_spans[0].timestamp == parent.block_info.timestamp + self.cfg.block_time {
                return self
                    .pop_next_batch(parent)
                    .ok_or(anyhow!("failed to pop next batch from span batch").into());
            }
            // Parent block does not match the next batch.
            // Means the previously returned batch is invalid.
            // Drop cached batches and find another batch.
            warn!(
                "Parent block does not match the next batch. Dropping {} cached batches.",
                self.next_spans.len()
            );
            self.next_spans.clear();
        }

        // If the epoch is advanced, update the l1 blocks.
        // Advancing epoch must be done after the pipeline successfully applies the entire span
        // batch to the chain.
        // Because the span batch can be reverted during processing the batch, then we must
        // preserve existing l1 blocks to verify the epochs of the next candidate batch.
        if !self.l1_blocks.is_empty() && parent.l1_origin.number > self.l1_blocks[0].number {
            for (i, block) in self.l1_blocks.iter().enumerate() {
                if parent.l1_origin.number == block.number {
                    self.l1_blocks.drain(0..i);
                    info!("Advancing epoch");
                    break;
                }
            }
            // If the origin of the parent block is not included, we must advance the origin.
        }

        // NOTE: The origin is used to determine if it's behind.
        // It is the future origin that gets saved into the l1 blocks array.
        // We always update the origin of this stage if it's not the same so
        // after the update code runs, this is consistent.
        let origin_behind =
            self.prev.origin().map_or(true, |origin| origin.number < parent.l1_origin.number);

        // Advance the origin if needed.
        // The entire pipeline has the same origin.
        // Batches prior to the l1 origin of the l2 safe head are not accepted.
        if self.origin != self.prev.origin().copied() {
            self.origin = self.prev.origin().cloned();
            if !origin_behind {
                let origin = self.origin.as_ref().ok_or_else(|| anyhow!("missing origin"))?;
                self.l1_blocks.push(*origin);
            } else {
                // This is to handle the special case of startup.
                // At startup, the batch queue is reset and includes the
                // l1 origin. That is the only time where immediately after
                // reset is called, the origin behind is false.
                self.l1_blocks.clear();
            }
            info!("Advancing batch queue origin: {:?}", self.origin);
        }

        // Load more data into the batch queue.
        let mut out_of_data = false;
        match self.prev.next_batch().await {
            Ok(b) => {
                if !origin_behind {
                    self.add_batch(b, parent).await.ok();
                } else {
                    warn!("Dropping batch: Origin is behind");
                }
            }
            Err(StageError::Eof) => out_of_data = true,
            Err(e) => return Err(e),
        }

        // Skip adding the data unless up to date with the origin,
        // but still fully empty the previous stages.
        if origin_behind {
            if out_of_data {
                return Err(StageError::Eof);
            }
            return Err(StageError::NotEnoughData);
        }

        // Attempt to derive more batches.
        let batch = match self.derive_next_batch(out_of_data, parent).await {
            Ok(b) => b,
            Err(e) => match e {
                StageError::Eof => {
                    if out_of_data {
                        return Err(StageError::Eof);
                    }
                    return Err(StageError::NotEnoughData);
                }
                _ => return Err(e),
            },
        };

        // If the next batch is derived from the span batch, it's the last batch of the span.
        // For singular batches, the span batch cache should be empty.
        match batch {
            Batch::Single(sb) => Ok(sb),
            Batch::Span(sb) => {
                let batches = sb.get_singular_batches(&self.l1_blocks, parent).map_err(|e| {
                    StageError::Custom(anyhow!(
                        "Could not get singular batches from span batch: {e}"
                    ))
                })?;
                self.next_spans = batches;
                let nb = self
                    .pop_next_batch(parent)
                    .ok_or_else(|| anyhow!("failed to pop next batch from span batch"))?;
                Ok(nb)
            }
        }
    }

    /// Returns if the previous batch was the last in the span.
    fn is_last_in_span(&self) -> bool {
        self.next_spans.is_empty()
    }
}

impl<P, BF> OriginProvider for BatchQueue<P, BF>
where
    P: BatchQueueProvider + OriginProvider + Debug,
    BF: L2ChainProvider + Debug,
{
    fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<P, BF> ResettableStage for BatchQueue<P, BF>
where
    P: BatchQueueProvider + OriginProvider + Send + Debug,
    BF: L2ChainProvider + Send + Debug,
{
    async fn reset(&mut self, base: BlockInfo, _: &SystemConfig) -> StageResult<()> {
        // Copy over the Origin from the next stage.
        // It is set in the engine queue (two stages away)
        // such that the L2 Safe Head origin is the progress.
        self.origin = Some(base);
        self.batches.clear();
        // Include the new origin as an origin to build on.
        // This is only for the initialization case.
        // During normal resets we will later throw out this block.
        self.l1_blocks.clear();
        self.l1_blocks.push(base);
        self.next_spans.clear();
        Err(StageError::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        stages::{
            channel_reader::BatchReader,
            test_utils::{CollectingLayer, MockBatchQueueProvider, TraceStorage},
        },
        traits::test_utils::MockBlockFetcher,
        types::{
            BatchType, BlockID, Genesis, L1BlockInfoBedrock, L1BlockInfoTx, L2ExecutionPayload,
            L2ExecutionPayloadEnvelope,
        },
    };
    use alloc::vec;
    use alloy_primitives::{address, b256, Address, Bytes, TxKind, B256, U256};
    use alloy_rlp::{BytesMut, Encodable};
    use miniz_oxide::deflate::compress_to_vec_zlib;
    use op_alloy_consensus::{OpTxType, TxDeposit};
    use tracing::Level;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    fn new_batch_reader() -> BatchReader {
        let raw_data = include_bytes!("../../testdata/raw_batch.hex");
        let mut typed_data = vec![BatchType::Span as u8];
        typed_data.extend_from_slice(raw_data.as_slice());
        let compressed = compress_to_vec_zlib(typed_data.as_slice(), 5);
        BatchReader::from(compressed)
    }

    #[tokio::test]
    async fn test_derive_next_batch_missing_origin() {
        let data = vec![Ok(Batch::Single(SingleBatch::default()))];
        let cfg = Arc::new(RollupConfig::default());
        let mock = MockBatchQueueProvider::new(data);
        let fetcher = MockBlockFetcher::default();
        let mut bq = BatchQueue::new(cfg, mock, fetcher);
        let parent = L2BlockInfo::default();
        let result = bq.derive_next_batch(false, parent).await.unwrap_err();
        assert_eq!(result, StageError::MissingOrigin);
    }

    #[tokio::test]
    async fn test_next_batch_not_enough_data() {
        let mut reader = new_batch_reader();
        let cfg = Arc::new(RollupConfig::default());
        let batch = reader.next_batch(cfg.as_ref()).unwrap();
        let mock = MockBatchQueueProvider::new(vec![Ok(batch)]);
        let fetcher = MockBlockFetcher::default();
        let mut bq = BatchQueue::new(cfg, mock, fetcher);
        let res = bq.next_batch(L2BlockInfo::default()).await.unwrap_err();
        assert_eq!(res, StageError::NotEnoughData);
        assert!(bq.is_last_in_span());
    }

    #[tokio::test]
    async fn test_next_batch_origin_behind() {
        let mut reader = new_batch_reader();
        let cfg = Arc::new(RollupConfig::default());
        let mut batch_vec: Vec<StageResult<Batch>> = vec![];
        while let Some(batch) = reader.next_batch(cfg.as_ref()) {
            batch_vec.push(Ok(batch));
        }
        let mut mock = MockBatchQueueProvider::new(batch_vec);
        mock.origin = Some(BlockInfo::default());
        let fetcher = MockBlockFetcher::default();
        let mut bq = BatchQueue::new(cfg, mock, fetcher);
        let parent = L2BlockInfo {
            l1_origin: BlockID { number: 10, ..Default::default() },
            ..Default::default()
        };
        let res = bq.next_batch(parent).await.unwrap_err();
        assert_eq!(res, StageError::NotEnoughData);
    }

    #[tokio::test]
    async fn test_next_batch_missing_origin() {
        let trace_store: TraceStorage = Default::default();
        let layer = CollectingLayer::new(trace_store.clone());
        tracing_subscriber::Registry::default().with(layer).init();

        let mut reader = new_batch_reader();
        let payload_block_hash =
            b256!("4444444444444444444444444444444444444444444444444444444444444444");
        let cfg = Arc::new(RollupConfig {
            delta_time: Some(0),
            block_time: 100,
            max_sequencer_drift: 10000000,
            seq_window_size: 10000000,
            genesis: Genesis {
                l2: BlockID { number: 8, hash: payload_block_hash },
                l1: BlockID { number: 16988980031808077784, ..Default::default() },
                ..Default::default()
            },
            ..Default::default()
        });
        let mut batch_vec: Vec<StageResult<Batch>> = vec![];
        let mut batch_txs: Vec<Bytes> = vec![];
        let mut second_batch_txs: Vec<Bytes> = vec![];
        while let Some(batch) = reader.next_batch(cfg.as_ref()) {
            // assert_eq!(batch, Batch::Span(Default::default()));
            if let Batch::Span(span) = &batch {
                let bys = span.batches[0]
                    .transactions
                    .iter()
                    .cloned()
                    .map(|tx| tx.0)
                    .collect::<Vec<Bytes>>();
                let sbys = span.batches[1]
                    .transactions
                    .iter()
                    .cloned()
                    .map(|tx| tx.0)
                    .collect::<Vec<Bytes>>();
                second_batch_txs.extend(sbys);
                batch_txs.extend(bys);
            }
            batch_vec.push(Ok(batch));
        }
        // Insert a deposit transaction in the front of the second batch txs
        let expected = L1BlockInfoBedrock {
            number: 16988980031808077784,
            time: 1697121143,
            base_fee: 10419034451,
            block_hash: b256!("392012032675be9f94aae5ab442de73c5f4fb1bf30fa7dd0d2442239899a40fc"),
            sequence_number: 4,
            batcher_address: address!("6887246668a3b87f54deb3b94ba47a6f63f32985"),
            l1_fee_overhead: U256::from(0xbc),
            l1_fee_scalar: U256::from(0xa6fe0),
        };
        let deposit_tx_calldata: Bytes = L1BlockInfoTx::Bedrock(expected).encode_calldata();
        let tx = TxDeposit {
            source_hash: B256::left_padding_from(&[0xde, 0xad]),
            from: Address::left_padding_from(&[0xbe, 0xef]),
            mint: Some(1),
            gas_limit: 2,
            to: TxKind::Call(Address::left_padding_from(&[3])),
            value: U256::from(4_u64),
            input: deposit_tx_calldata,
            is_system_transaction: false,
        };
        let mut buf = BytesMut::new();
        tx.encode(&mut buf);
        let prefixed = [&[OpTxType::Deposit as u8], &buf[..]].concat();
        second_batch_txs.insert(0, Bytes::copy_from_slice(&prefixed));
        let mut mock = MockBatchQueueProvider::new(batch_vec);
        let origin_check =
            b256!("8527cdb6f601acf9b483817abd1da92790c92b19000000000000000000000000");
        mock.origin = Some(BlockInfo {
            number: 16988980031808077784,
            // 1639845645
            timestamp: 1639845845,
            parent_hash: Default::default(),
            hash: origin_check,
        });
        let origin = mock.origin;

        let parent_check =
            b256!("01ddf682e2f8a6f10c2207e02322897e65317196000000000000000000000000");
        let block_nine = L2BlockInfo {
            block_info: BlockInfo {
                number: 9,
                timestamp: 1639845645,
                parent_hash: parent_check,
                hash: origin_check,
            },
            ..Default::default()
        };
        let block_seven = L2BlockInfo {
            block_info: BlockInfo {
                number: 7,
                timestamp: 1639845745,
                parent_hash: parent_check,
                hash: origin_check,
            },
            ..Default::default()
        };
        let payload = L2ExecutionPayloadEnvelope {
            parent_beacon_block_root: None,
            execution_payload: L2ExecutionPayload {
                block_number: 8,
                block_hash: payload_block_hash,
                transactions: batch_txs,
                ..Default::default()
            },
        };
        let second = L2ExecutionPayloadEnvelope {
            parent_beacon_block_root: None,
            execution_payload: L2ExecutionPayload {
                block_number: 9,
                block_hash: payload_block_hash,
                transactions: second_batch_txs,
                ..Default::default()
            },
        };
        let fetcher = MockBlockFetcher {
            blocks: vec![block_nine, block_seven],
            payloads: vec![payload, second],
        };
        let mut bq = BatchQueue::new(cfg, mock, fetcher);
        let parent = L2BlockInfo {
            block_info: BlockInfo {
                number: 9,
                timestamp: 1639845745,
                parent_hash: parent_check,
                hash: origin_check,
            },
            l1_origin: BlockID { number: 16988980031808077784, hash: origin_check },
            ..Default::default()
        };
        let res = bq.next_batch(parent).await.unwrap_err();
        let logs = trace_store.get_by_level(Level::INFO);
        assert_eq!(logs.len(), 5);
        let str = alloc::format!("Advancing batch queue origin: {:?}", origin);
        assert!(logs[0].contains(&str));
        assert!(logs[1].contains("Deriving next batch for epoch: 16988980031808077784"));
        assert!(logs[2].contains("Next batch found:"));
        let warns = trace_store.get_by_level(Level::WARN);
        assert_eq!(warns.len(), 0);
        let str = "Could not get singular batches from span batch: Missing L1 origin";
        assert_eq!(res, StageError::Custom(anyhow::anyhow!(str)));
    }

    #[tokio::test]
    async fn test_batch_queue_empty_bytes() {
        let data = vec![Ok(Batch::Single(SingleBatch::default()))];
        let cfg = Arc::new(RollupConfig::default());
        let mock = MockBatchQueueProvider::new(data);
        let fetcher = MockBlockFetcher::default();
        let mut bq = BatchQueue::new(cfg, mock, fetcher);
        let parent = L2BlockInfo::default();
        let result = bq.next_batch(parent).await;
        assert!(result.is_ok());
        // assert_eq!(result, Err(StageError::NotEnoughData));
    }
}
