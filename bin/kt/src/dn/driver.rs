//! Pipeline runner.
//!
//! The runner that executes the pipeline and validates the output given the test fixtures.

use super::{pipeline::RunnerPipeline, providers::FixtureL2Provider, LocalDerivationFixture};
use anyhow::{anyhow, Result};
use kona_derive::{
    errors::StageError,
    pipeline::StepResult,
    traits::{L2ChainProvider, Pipeline},
};
use tracing::{debug, error, info, trace, warn};

const LOG_TARGET: &str = "runner";

/// Runs the pipeline.
pub(crate) async fn run(
    mut pipeline: RunnerPipeline,
    fixture: LocalDerivationFixture,
) -> Result<()> {
    let mut cursor = *fixture
        .l2_block_infos
        .get(&fixture.l2_cursor_start)
        .ok_or_else(|| anyhow!("No block info found"))?;
    let mut l2_provider = FixtureL2Provider::from(fixture.clone());
    let mut advance_cursor_flag = false;
    let end = fixture.l2_cursor_end;
    loop {
        if advance_cursor_flag {
            match l2_provider.l2_block_info_by_number(cursor.block_info.number + 1).await {
                Ok(bi) => {
                    cursor = bi;
                    advance_cursor_flag = false;
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to fetch next pending l2 safe head: {}, err: {:?}", cursor.block_info.number + 1, e);
                    // We don't need to step on the pipeline if we failed to fetch the next pending
                    // l2 safe head.
                    continue;
                }
            }
        }
        if (cursor.block_info.number + 1) >= end {
            info!(target: LOG_TARGET, "All payload attributes successfully validated");
            break;
        }
        trace!(target: LOG_TARGET, "Stepping on cursor block number: {}", cursor.block_info.number);
        match pipeline.step(cursor).await {
            StepResult::PreparedAttributes => trace!(target: "loop", "Prepared attributes"),
            StepResult::AdvancedOrigin => trace!(target: "loop", "Advanced origin"),
            StepResult::OriginAdvanceErr(e) => {
                warn!(target: "loop", "Could not advance origin: {:?}", e)
            }
            StepResult::StepFailed(e) => match e {
                StageError::NotEnoughData => {
                    debug!(target: "loop", "Not enough data to step derivation pipeline");
                }
                _ => {
                    error!(target: "loop", "Error stepping derivation pipeline: {:?}", e);
                }
            },
        }

        // Take the next attributes from the pipeline.
        let Some(attributes) = pipeline.next() else {
            continue;
        };

        // Validate the attributes against the reference.
        let Some(expected) = fixture.l2_payloads.get(&(cursor.block_info.number + 1)) else {
            return Err(anyhow!("No expected payload found"));
        };
        if attributes.attributes != *expected {
            error!(target: LOG_TARGET, "Attributes do not match expected");
            debug!(target: LOG_TARGET, "Expected: {:?}", expected);
            debug!(target: LOG_TARGET, "Actual: {:?}", attributes);
            return Err(anyhow!("Attributes do not match expected"));
        }
        advance_cursor_flag = true;
    }
    Ok(())
}
