//! The driver of the Derivation Pipeline.

use alloc::vec::Vec;
use alloy_consensus::{BlockBody, Header, Sealable, Sealed};
use alloy_primitives::B256;
use alloy_rlp::Decodable;
use core::fmt::Debug;
use kona_derive::{
    errors::{PipelineError, PipelineErrorKind},
    types::Signal,
};
use op_alloy_consensus::{OpBlock, OpTxEnvelope, OpTxType};
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::L2BlockInfo;
use op_alloy_rpc_types_engine::OpAttributesWithParent;
use tracing::{error, info, warn};

use crate::{DriverError, DriverResult, Executor, ExecutorConstructor, Pipeline, SyncCursor};

/// The Rollup Driver entrypoint.
#[derive(Debug)]
pub struct Driver<E, EC, P>
where
    E: Executor + Send + Sync + Debug,
    EC: ExecutorConstructor<E> + Send + Sync + Debug,
    P: Pipeline + Send + Sync + Debug,
{
    /// Marker for the executor.
    _marker: core::marker::PhantomData<E>,
    /// A pipeline abstraction.
    pipeline: P,
    /// Cursor to keep track of the L2 tip
    cursor: SyncCursor,
    /// Executor constructor.
    executor: EC,
}

impl<E, EC, P> Driver<E, EC, P>
where
    E: Executor + Send + Sync + Debug,
    EC: ExecutorConstructor<E> + Send + Sync + Debug,
    P: Pipeline + Send + Sync + Debug,
{
    /// Creates a new [Driver].
    pub const fn new(cursor: SyncCursor, executor: EC, pipeline: P) -> Self {
        Self { _marker: core::marker::PhantomData, cursor, executor, pipeline }
    }

    /// Returns the current L2 safe head.
    pub const fn l2_safe_head(&self) -> &L2BlockInfo {
        self.cursor.l2_safe_head()
    }

    /// Returns the header of the L2 safe head.
    pub const fn l2_safe_head_header(&self) -> &Sealed<Header> {
        self.cursor.l2_safe_head_header()
    }

    /// Returns the output root of the L2 safe head.
    pub const fn l2_safe_head_output_root(&self) -> &B256 {
        self.cursor.l2_safe_head_output_root()
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
        mut target: u64,
    ) -> DriverResult<(u64, B256), E::Error> {
        loop {
            // Check if we have reached the target block number.
            if self.l2_safe_head().block_info.number >= target {
                info!(target: "client", "Derivation complete, reached L2 safe head.");
                return Ok((
                    self.l2_safe_head().block_info.number,
                    *self.l2_safe_head_output_root(),
                ));
            }

            let OpAttributesWithParent { mut attributes, .. } = match self
                .pipeline
                .produce_payload(*self.l2_safe_head())
                .await
            {
                Ok(attrs) => attrs,
                Err(PipelineErrorKind::Critical(PipelineError::EndOfSource)) => {
                    warn!(target: "client", "Exhausted data source; Halting derivation and using current safe head.");

                    // Adjust the target block number to the current safe head, as no more blocks
                    // can be produced.
                    target = self.l2_safe_head().block_info.number;
                    continue;
                }
                Err(e) => {
                    error!(target: "client", "Failed to produce payload: {:?}", e);
                    return Err(DriverError::Pipeline(e));
                }
            };

            let mut executor = self.executor.new_executor(self.l2_safe_head_header().clone());
            let header = match executor.execute_payload(attributes.clone()) {
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
                        executor = self.executor.new_executor(self.l2_safe_head_header().clone());
                        match executor.execute_payload(attributes.clone()) {
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

            // Update the safe head.
            self.cursor.l2_safe_head = L2BlockInfo::from_block_and_genesis(
                &block,
                &self.pipeline.rollup_config().genesis,
            )?;
            self.cursor.l2_safe_head_header = header.clone().seal_slow();
            self.cursor.l2_safe_head_output_root =
                executor.compute_output_root().map_err(DriverError::Executor)?;
        }
    }
}
