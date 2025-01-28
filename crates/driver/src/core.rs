//! The driver of the kona derivation pipeline.

use crate::{DriverError, DriverResult, EngineController, L2ChainHeads, SafetyLabel};
use alloc::vec::Vec;
use alloy_consensus::{BlockBody, Header, Sealed};
use alloy_rlp::Decodable;
use alloy_rpc_types_engine::{PayloadStatus, PayloadStatusEnum};
use core::fmt::Debug;
use kona_derive::{traits::Pipeline, types::Signal};
use maili_protocol::L2BlockInfo;
use op_alloy_consensus::{OpBlock, OpTxEnvelope, OpTxType};
use op_alloy_rpc_types_engine::OpAttributesWithParent;

/// The Rollup Driver.
///
/// The [Driver] is a low-level interface, responsible for progressing and tracking the L2 chain. It exposes interfaces
/// to the subroutines that drive the extension of both the (local view of the) unsafe and safe L2 chain.
///
/// The sub-routines for the driver are:
/// 1. Safe chain derivation
/// 2. Payload execution (unsafe + safe)
/// 3. Local [L2ChainHeads] state updates
///
/// Currently does not support:
/// - [L1 consolidation](https://specs.optimism.io/protocol/derivation.html#l1-consolidation-payload-attributes-matching)
/// - [Ingesting + processing unsafe payload attributes](https://specs.optimism.io/protocol/derivation.html#processing-unsafe-payload-attributes)
#[derive(Debug)]
pub struct Driver<DP, EC>
where
    DP: Pipeline + Send + Sync + Debug,
    EC: EngineController + Send + Sync + Debug,
{
    /// The derivation [Pipeline]
    pub pipeline: DP,
    /// The [EngineController]
    pub engine: EC,
    /// Local L2 chain heads
    pub heads: L2ChainHeads,
}

impl<DP, EC> Driver<DP, EC>
where
    DP: Pipeline + Send + Sync + Debug,
    EC: EngineController + Send + Sync + Debug,
{
    /// Creates a new [Driver].
    pub const fn new(pipeline: DP, engine: EC, heads: L2ChainHeads) -> Self {
        Self { pipeline, engine, heads }
    }

    /// Produces the next [OpAttributesWithParent], derived from L1 via the [Pipeline], that extends the current L2
    /// safe chain.
    pub async fn produce_safe_extension_payload(
        &mut self,
    ) -> DriverResult<OpAttributesWithParent, EC::Error> {
        // Important TODO to address before merge: We removed some code here that handled halting derivation
        // when the pipeline is exhausted. This is proofs-specific, and this crate shouldn't care. It needs
        // to be handled in upstream proof oracle pipeline.
        self.pipeline.produce_extension_payload(*self.heads.safe_head()).await.map_err(Into::into)
    }

    /// Executes an [OpAttributesWithParent] payload using the [Executor] interface. The consumer is responsible for
    /// forwarding the sealed block and its [SafetyLabel] to [Self::update_cursor].
    ///
    /// Steps:
    /// 1. Attempt initial execution of the [OpAttributesWithParent] verbatim.
    /// 2. If execution fails:
    ///     2.a. If Holocene is active, strip the payload down to only [TxDeposit] transactions, and retry.
    ///     2.b. If Holocene is not active, discard the [OpAttributesWithParent] gracefully and continue to the next.
    ///
    /// [TxDeposit]: maili_consensus::TxDeposit
    pub async fn execute_payload(
        &mut self,
        mut attributes_with_parent: OpAttributesWithParent,
    ) -> DriverResult<Option<Sealed<OpBlock>>, EC::Error> {
        // Attempt to build the block verbatim. If this round succeeds, immediately return the sealed header.
        if let Ok(fcu_resp) = self
            .engine
            .forkchoice_updated(self.heads.into(), Some(attributes_with_parent.attributes.clone()))
            .await
        {
            // Handle the FCU response.
            match fcu_resp.payload_status.status {
                PayloadStatusEnum::Valid => {}
                PayloadStatusEnum::Invalid { validation_error } => todo!("Throw error"),
                PayloadStatusEnum::Syncing | PayloadStatusEnum::Accepted => {
                    unimplemented!("Syncing and Accepted are not supported responses yet.")
                }
            }

            let payload_id = fcu_resp.payload_id.ok_or(DriverError::Engine(todo!()));

            // return Ok(Some(payload_and_header_to_block::<EC>(attributes_with_parent, header)?));
            todo!()
        }

        if self
            .pipeline
            .rollup_config()
            .is_holocene_active(attributes_with_parent.attributes.payload_attributes.timestamp)
        {
            // Retry with a deposit-only block.
            warn!(target: "driver", "Flushing current channel and retrying execution of deposit only block");

            // Flush the current batch and channel - if a block was replaced with a
            // deposit-only block due to execution failure, the
            // batch and channel it is contained in is forwards
            // invalidated.
            self.pipeline.signal(Signal::FlushChannel).await?;

            // Strip out all transactions that are not deposits.
            attributes_with_parent.attributes.transactions =
                attributes_with_parent.attributes.transactions.map(|txs| {
                    txs.into_iter()
                        .filter(|tx| (!tx.is_empty() && tx[0] == OpTxType::Deposit as u8))
                        .collect::<Vec<_>>()
                });

            // Retry building the block. If this fails, throw a critical error.
            match self
                .engine
                .forkchoice_updated(
                    self.heads.into(),
                    Some(attributes_with_parent.attributes.clone()),
                )
                .await
            {
                Ok(header) => {
                    Ok(Some(payload_and_header_to_block::<EC>(attributes_with_parent, header)?))
                }
                Err(e) => {
                    error!(
                        target: "driver",
                        "Critical - Failed to execute deposit-only block: {e}",
                    );
                    Err(DriverError::Engine(e))
                }
            }
        } else {
            // Discard the payload if holocene is not active and the initial execution of the payload attributes fails.
            Ok(None)
        }
    }

    /// Updates the local [L2ChainHeads] with the new block for the given [SafetyLabel].
    ///
    /// ## Safety
    /// - Assumes that the block has already been validated.
    pub async fn advance_head(
        &mut self,
        block: Sealed<OpBlock>,
        safety_label: SafetyLabel,
    ) -> DriverResult<(), EC::Error> {
        let l2_info =
            L2BlockInfo::from_block_and_genesis(&block, &self.pipeline.rollup_config().genesis)?;
        self.heads.advance(safety_label, l2_info);
        Ok(())
    }

    async fn build_block(
        &mut self,
        attributes_with_parent: OpAttributesWithParent,
    ) -> DriverResult<Sealed<OpBlock>, EC::Error> {
        let fcu_response = self
            .engine
            .forkchoice_updated(
                self.heads.clone().into(),
                Some(attributes_with_parent.attributes.clone()),
            )
            .await
            .unwrap();

        match fcu_response.payload_status.status {
            PayloadStatusEnum::Valid => { /* continue */ }
            PayloadStatusEnum::Invalid { validation_error } => todo!(),
            PayloadStatusEnum::Syncing | PayloadStatusEnum::Accepted => {
                unimplemented!(
                    "`Syncing` and `Accepted` are not supported Engine API responses yet."
                )
            }
        }

        let payload_id =
            fcu_response.payload_id.expect("Must have a payload ID; Attributes were sent.");

        let payload_response = self.engine.get_payload(payload_id).await.unwrap();

        todo!()
    }
}

/// Converts a [OpAttributesWithParent] and sealed [Header] into an [OpBlock].
fn payload_and_header_to_block<EC: EngineController>(
    attributes_with_parent: OpAttributesWithParent,
    new_head: Sealed<Header>,
) -> DriverResult<Sealed<OpBlock>, EC::Error> {
    let block = OpBlock {
        header: new_head.inner().clone(),
        body: BlockBody {
            transactions: attributes_with_parent
                .attributes
                .transactions
                .unwrap_or_default()
                .into_iter()
                .map(|tx| OpTxEnvelope::decode(&mut tx.as_ref()).map_err(DriverError::Rlp))
                .collect::<DriverResult<Vec<OpTxEnvelope>, EC::Error>>()?,
            ommers: Vec::new(),
            withdrawals: None,
        },
    };
    Ok(Sealed::new_unchecked(block, new_head.hash()))
}
