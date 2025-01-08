//! The driver of the kona derivation pipeline.

use alloc::vec::Vec;
use alloy_consensus::{BlockBody, Sealable};
use alloy_primitives::B256;
use alloy_rlp::Decodable;
use core::fmt::Debug;
use kona_derive::{
    errors::{PipelineError, PipelineErrorKind},
    traits::{Pipeline, SignalReceiver},
    types::Signal,
};
use maili_protocol::L2BlockInfo;
use op_alloy_consensus::{OpBlock, OpTxEnvelope, OpTxType};
use op_alloy_genesis::RollupConfig;
use op_alloy_rpc_types_engine::OpAttributesWithParent;

use crate::{DriverError, DriverPipeline, DriverResult, Executor, PipelineCursor, TipCursor};

/// The Rollup Driver entrypoint.
#[derive(Debug)]
pub struct Driver<E, DP, P>
where
    E: Executor + Send + Sync + Debug,
    DP: DriverPipeline<P> + Send + Sync + Debug,
    P: Pipeline + SignalReceiver + Send + Sync + Debug,
{
    /// Marker for the executor.
    _marker: core::marker::PhantomData<E>,
    /// Marker for the pipeline.
    _marker2: core::marker::PhantomData<P>,
    /// A pipeline abstraction.
    pub pipeline: DP,
    /// Cursor to keep track of the L2 tip
    pub cursor: PipelineCursor,
    /// The Executor.
    pub executor: E,
}

impl<E, DP, P> Driver<E, DP, P>
where
    E: Executor + Send + Sync + Debug,
    DP: DriverPipeline<P> + Send + Sync + Debug,
    P: Pipeline + SignalReceiver + Send + Sync + Debug,
{
    /// Creates a new [Driver].
    pub const fn new(cursor: PipelineCursor, executor: E, pipeline: DP) -> Self {
        Self {
            _marker: core::marker::PhantomData,
            _marker2: core::marker::PhantomData,
            pipeline,
            cursor,
            executor,
        }
    }

    /// Waits until the executor is ready.
    pub async fn wait_for_executor(&mut self) {
        self.executor.wait_until_ready().await;
    }

    /// Advances the derivation pipeline to the target block number.
    ///
    /// ## Takes
    /// - `cfg`: The rollup configuration.
    /// - `target`: The target block number.
    ///
    /// ## Returns
    /// - `Ok((number, output_root))` - A tuple containing the number of the produced block and the
    ///   output root.
    /// - `Err(e)` - An error if the block could not be produced.
    pub async fn advance_to_target(
        &mut self,
        cfg: &RollupConfig,
        mut target: Option<u64>,
    ) -> DriverResult<(u64, B256), E::Error> {
        loop {
            // Check if we have reached the target block number.
            if let Some(tb) = target {
                if self.cursor.l2_safe_head().block_info.number >= tb {
                    info!(target: "client", "Derivation complete, reached L2 safe head.");
                    return Ok((
                        self.cursor.l2_safe_head().block_info.number,
                        *self.cursor.l2_safe_head_output_root(),
                    ));
                }
            }

            let OpAttributesWithParent { mut attributes, .. } = match self
                .pipeline
                .produce_payload(*self.cursor.l2_safe_head())
                .await
            {
                Ok(attrs) => attrs,
                Err(PipelineErrorKind::Critical(PipelineError::EndOfSource)) => {
                    warn!(target: "client", "Exhausted data source; Halting derivation and using current safe head.");

                    // Adjust the target block number to the current safe head, as no more blocks
                    // can be produced.
                    if target.is_some() {
                        target = Some(self.cursor.l2_safe_head().block_info.number);
                    };
                    continue;
                }
                Err(e) => {
                    error!(target: "client", "Failed to produce payload: {:?}", e);
                    return Err(DriverError::Pipeline(e));
                }
            };

            self.executor.update_safe_head(self.cursor.l2_safe_head_header().clone());
            let header = match self.executor.execute_payload(attributes.clone()).await {
                Ok(header) => header,
                Err(e) => {
                    error!(target: "client", "Failed to execute L2 block: {}", e);

                    if cfg.is_holocene_active(attributes.payload_attributes.timestamp) {
                        // Retry with a deposit-only block.
                        warn!(target: "client", "Flushing current channel and retrying deposit only block");

                        // Flush the current batch and channel - if a block was replaced with a
                        // deposit-only block due to execution failure, the
                        // batch and channel it is contained in is forwards
                        // invalidated.
                        self.pipeline.signal(Signal::FlushChannel).await?;

                        // Strip out all transactions that are not deposits.
                        attributes.transactions = attributes.transactions.map(|txs| {
                            txs.into_iter()
                                .filter(|tx| (!tx.is_empty() && tx[0] == OpTxType::Deposit as u8))
                                .collect::<Vec<_>>()
                        });

                        // Retry the execution.
                        self.executor.update_safe_head(self.cursor.l2_safe_head_header().clone());
                        match self.executor.execute_payload(attributes.clone()).await {
                            Ok(header) => header,
                            Err(e) => {
                                error!(
                                    target: "client",
                                    "Critical - Failed to execute deposit-only block: {e}",
                                );
                                return Err(DriverError::Executor(e));
                            }
                        }
                    } else {
                        // Pre-Holocene, discard the block if execution fails.
                        continue;
                    }
                }
            };

            // Construct the block.
            let block = OpBlock {
                header: header.clone(),
                body: BlockBody {
                    transactions: attributes
                        .transactions
                        .unwrap_or_default()
                        .into_iter()
                        .map(|tx| OpTxEnvelope::decode(&mut tx.as_ref()).map_err(DriverError::Rlp))
                        .collect::<DriverResult<Vec<OpTxEnvelope>, E::Error>>()?,
                    ommers: Vec::new(),
                    withdrawals: None,
                },
            };

            // Get the pipeline origin and update the cursor.
            let origin = self.pipeline.origin().ok_or(PipelineError::MissingOrigin.crit())?;
            let l2_info = L2BlockInfo::from_block_and_genesis(
                &block,
                &self.pipeline.rollup_config().genesis,
            )?;
            let cursor = TipCursor::new(
                l2_info,
                header.clone().seal_slow(),
                self.executor.compute_output_root().map_err(DriverError::Executor)?,
            );
            self.cursor.advance(origin, cursor);
        }
    }
}
