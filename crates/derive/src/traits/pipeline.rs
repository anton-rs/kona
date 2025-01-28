//! Defines the interface for the core derivation pipeline.

use super::SignalReceiver;
use crate::{
    errors::{PipelineError, PipelineErrorKind, ResetError},
    traits::OriginProvider,
    types::{ActivationSignal, ResetSignal, StepResult},
};
use alloc::boxed::Box;
use async_trait::async_trait;
use core::iter::Iterator;
use maili_genesis::{RollupConfig, SystemConfig};
use maili_protocol::L2BlockInfo;
use op_alloy_rpc_types_engine::OpAttributesWithParent;

/// This trait defines the interface for interacting with the derivation pipeline.
#[async_trait]
pub trait Pipeline:
    OriginProvider + SignalReceiver + Iterator<Item = OpAttributesWithParent>
{
    /// Peeks at the next [OpAttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&OpAttributesWithParent>;

    /// Attempts to progress the pipeline.
    async fn step(&mut self, l2_safe_head: L2BlockInfo) -> StepResult;

    /// Returns the rollup config.
    fn rollup_config(&self) -> &RollupConfig;

    /// Returns the [SystemConfig] by L2 number.
    async fn system_config_by_number(
        &mut self,
        number: u64,
    ) -> Result<SystemConfig, PipelineErrorKind>;

    /// Produces the next [OpAttributesWithParent] from the pipeline, consuming the current safe head [L2BlockInfo]
    /// to orient the pipeline.
    async fn produce_extension_payload(
        &mut self,
        l2_safe_head: L2BlockInfo,
    ) -> Result<OpAttributesWithParent, PipelineErrorKind> {
        // As we start the safe head at the disputed block's parent, we step the pipeline until the
        // first attributes are produced. All batches at and before the safe head will be
        // dropped, so the first payload will always be the disputed one.
        loop {
            match self.step(l2_safe_head).await {
                StepResult::PreparedAttributes => {
                    info!(target: "client_derivation_driver", "Stepped derivation pipeline")
                }
                StepResult::AdvancedOrigin => {
                    info!(target: "client_derivation_driver", "Advanced origin")
                }
                StepResult::OriginAdvanceErr(e) | StepResult::StepFailed(e) => {
                    // Break the loop unless the error signifies that there is not enough data to
                    // complete the current step. In this case, we retry the step to see if other
                    // stages can make progress.
                    match e {
                        PipelineErrorKind::Temporary(_) => {
                            trace!(target: "client_derivation_driver", "Failed to step derivation pipeline temporarily: {:?}", e);
                            continue;
                        }
                        PipelineErrorKind::Reset(e) => {
                            warn!(target: "client_derivation_driver", "Failed to step derivation pipeline due to reset: {:?}", e);
                            let system_config = self
                                .system_config_by_number(l2_safe_head.block_info.number)
                                .await?;

                            if matches!(e, ResetError::HoloceneActivation) {
                                let l1_origin =
                                    self.origin().ok_or(PipelineError::MissingOrigin.crit())?;
                                self.signal(
                                    ActivationSignal {
                                        l2_safe_head,
                                        l1_origin,
                                        system_config: Some(system_config),
                                    }
                                    .signal(),
                                )
                                .await?;
                            } else {
                                // Flushes cache if a reorg is detected.
                                if matches!(e, ResetError::ReorgDetected(_, _)) {
                                    // self.flush();
                                    todo!("Signal reorg to consumer");
                                }

                                // Reset the pipeline to the initial L2 safe head and L1 origin,
                                // and try again.
                                let l1_origin =
                                    self.origin().ok_or(PipelineError::MissingOrigin.crit())?;
                                self.signal(
                                    ResetSignal {
                                        l2_safe_head,
                                        l1_origin,
                                        system_config: Some(system_config),
                                    }
                                    .signal(),
                                )
                                .await?;
                            }
                        }
                        PipelineErrorKind::Critical(_) => {
                            warn!(target: "client_derivation_driver", "Failed to step derivation pipeline: {:?}", e);
                            return Err(e);
                        }
                    }
                }
            }

            if let Some(attrs) = self.next() {
                return Ok(attrs);
            }
        }
    }
}
