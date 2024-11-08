//! Abstracts the derivation pipeline from the driver.

use alloc::boxed::Box;
use async_trait::async_trait;
use op_alloy_protocol::L2BlockInfo;
use op_alloy_rpc_types_engine::OpAttributesWithParent;

use kona_derive::{
    errors::{PipelineError, PipelineErrorKind, ResetError},
    traits::{Pipeline, SignalReceiver},
    types::{ActivationSignal, ResetSignal, StepResult},
};
use tracing::{info, warn};

/// The Driver's Pipeline
///
/// A high-level abstraction for the driver's derivation pipeline.
#[async_trait]
pub trait DriverPipeline<P>
where
    P: Pipeline + SignalReceiver,
{
    /// Returns the inner Pipeline.
    fn inner(&mut self) -> &mut P;

    /// Flushes any cache on re-org.
    fn flush(&self);

    /// Produces the disputed [OpAttributesWithParent] payload, directly after the given
    /// starting l2 safe head.
    async fn produce_payload(
        &mut self,
        l2_safe_head: L2BlockInfo,
    ) -> Result<OpAttributesWithParent, PipelineErrorKind> {
        // As we start the safe head at the disputed block's parent, we step the pipeline until the
        // first attributes are produced. All batches at and before the safe head will be
        // dropped, so the first payload will always be the disputed one.
        loop {
            match self.inner().step(l2_safe_head).await {
                StepResult::PreparedAttributes => {
                    info!(target: "client_derivation_driver", "Stepped derivation pipeline")
                }
                StepResult::AdvancedOrigin => {
                    info!(target: "client_derivation_driver", "Advanced origin")
                }
                StepResult::OriginAdvanceErr(e) | StepResult::StepFailed(e) => {
                    warn!(target: "client_derivation_driver", "Failed to step derivation pipeline: {:?}", e);

                    // Break the loop unless the error signifies that there is not enough data to
                    // complete the current step. In this case, we retry the step to see if other
                    // stages can make progress.
                    match e {
                        PipelineErrorKind::Temporary(_) => continue,
                        PipelineErrorKind::Reset(e) => {
                            let system_config = self
                                .inner()
                                .system_config_by_number(l2_safe_head.block_info.number)
                                .await?;

                            if matches!(e, ResetError::HoloceneActivation) {
                                let l1_origin = self
                                    .inner()
                                    .origin()
                                    .ok_or(PipelineError::MissingOrigin.crit())?;
                                self.inner()
                                    .signal(
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
                                    self.flush();
                                }

                                // Reset the pipeline to the initial L2 safe head and L1 origin,
                                // and try again.
                                let l1_origin = self
                                    .inner()
                                    .origin()
                                    .ok_or(PipelineError::MissingOrigin.crit())?;
                                self.inner()
                                    .signal(
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
                        PipelineErrorKind::Critical(_) => return Err(e),
                    }
                }
            }

            if let Some(attrs) = self.inner().next() {
                return Ok(attrs);
            }
        }
    }
}