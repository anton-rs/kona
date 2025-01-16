//! Single chain sub-transition phase of the interop proof.

use super::FaultProofProgramError;
use crate::interop::util::fetch_l2_safe_head_hash;
use alloc::sync::Arc;
use alloy_consensus::Sealed;
use alloy_primitives::B256;
use core::fmt::Debug;
use kona_derive::errors::{PipelineError, PipelineErrorKind};
use kona_driver::{Driver, DriverError};
use kona_executor::{KonaHandleRegister, TrieDBProvider};
use kona_preimage::{HintWriterClient, PreimageOracleClient};
use kona_proof::{
    executor::KonaExecutor,
    l1::{OracleBlobProvider, OracleL1ChainProvider, OraclePipeline},
    l2::OracleL2ChainProvider,
    sync::new_pipeline_cursor,
    CachingOracle,
};
use kona_proof_interop::{BootInfo, OptimisticBlock, PreState, INVALID_TRANSITION_HASH};
use tracing::{error, info, warn};

/// Executes a sub-transition of the interop proof with the given [PreimageOracleClient] and
/// [HintWriterClient].
pub(crate) async fn sub_transition<P, H>(
    oracle: Arc<CachingOracle<P, H>>,
    handle_register: Option<
        KonaHandleRegister<
            OracleL2ChainProvider<CachingOracle<P, H>>,
            OracleL2ChainProvider<CachingOracle<P, H>>,
        >,
    >,
    boot: BootInfo,
    pre: PreState,
) -> Result<(), FaultProofProgramError>
where
    P: PreimageOracleClient + Send + Sync + Debug + Clone,
    H: HintWriterClient + Send + Sync + Debug + Clone,
{
    // Check if we can short-circuit the transition, if we are within padding.
    // TODO: Deduplicate claim check logic.
    if let PreState::TransitionState(ref transition_state) = pre {
        if transition_state.step >= transition_state.pre_state.output_roots.len() as u64 {
            info!(
                target: "interop_client",
                "No state transition required, transition state is already saturated."
            );

            return transition_and_check(pre, None, boot.claimed_post_state);
        }
    }

    // Fetch the L2 block hash of the current safe head.
    let safe_head_hash = fetch_l2_safe_head_hash(oracle.as_ref(), &pre).await?;

    // Instantiate the L1 EL + CL provider and the L2 EL provider.
    let mut l1_provider = OracleL1ChainProvider::new(boot.l1_head, oracle.clone());
    let mut l2_provider =
        OracleL2ChainProvider::new(safe_head_hash, boot.rollup_config.clone(), oracle.clone());
    let beacon = OracleBlobProvider::new(oracle.clone());

    // Fetch the safe head's block header.
    let safe_head = l2_provider
        .header_by_hash(safe_head_hash)
        .map(|header| Sealed::new_unchecked(header, safe_head_hash))?;

    // Translate the claimed timestamp to an L2 block number.
    let claimed_l2_block_number = boot.rollup_config.genesis.l2.number +
        ((boot.claimed_l2_timestamp - boot.rollup_config.genesis.l2_time) /
            boot.rollup_config.block_time);

    // If the claimed L2 block number is less than the safe head of the L2 chain, the claim is
    // invalid.
    if claimed_l2_block_number < safe_head.number {
        error!(
            target: "interop_client",
            "Claimed L2 block number {claimed} is less than the safe head {safe}",
            claimed = claimed_l2_block_number,
            safe = safe_head.number
        );
        return Err(FaultProofProgramError::InvalidClaim(
            boot.agreed_pre_state,
            boot.claimed_post_state,
        ));
    }

    // In the case where the agreed upon L2 pre-state is the same as the claimed L2 post-state,
    // trace extension is detected and we can skip the derivation and execution steps.
    if boot.agreed_pre_state == boot.claimed_post_state {
        info!(
            target: "interop_client",
            "Trace extension detected. State transition is already agreed upon.",
        );
        return Err(FaultProofProgramError::InvalidClaim(
            boot.agreed_pre_state,
            boot.claimed_post_state,
        ));
    }

    // Create a new derivation driver with the given boot information and oracle.
    let cursor =
        new_pipeline_cursor(&boot.rollup_config, safe_head, &mut l1_provider, &mut l2_provider)
            .await?;
    let cfg = Arc::new(boot.rollup_config.clone());
    let pipeline = OraclePipeline::new(
        cfg.clone(),
        cursor.clone(),
        oracle.clone(),
        beacon,
        l1_provider.clone(),
        l2_provider.clone(),
    );
    let executor = KonaExecutor::new(&cfg, l2_provider.clone(), l2_provider, handle_register, None);
    let mut driver = Driver::new(cursor, executor, pipeline);

    // Run the derivation pipeline until we are able to produce the output root of the claimed
    // L2 block.
    match driver.advance_to_target(&boot.rollup_config, Some(claimed_l2_block_number)).await {
        Ok((_, block_hash, output_root)) => {
            let optimistic_block = OptimisticBlock::new(block_hash, output_root);
            transition_and_check(pre, Some(optimistic_block), boot.claimed_post_state)?;

            info!(
                target: "interop_client",
                "Successfully validated progressed transition state claim with commitment {post_state_commitment}",
                post_state_commitment = boot.claimed_post_state
            );

            Ok(())
        }
        Err(DriverError::Pipeline(PipelineErrorKind::Critical(PipelineError::EndOfSource))) => {
            warn!(
                target: "interop_client",
                "Exhausted data source; Transitioning to invalid state."
            );

            if boot.claimed_post_state == INVALID_TRANSITION_HASH {
                Ok(())
            } else {
                Err(FaultProofProgramError::InvalidClaim(
                    INVALID_TRANSITION_HASH,
                    boot.claimed_post_state,
                ))
            }
        }
        Err(e) => {
            error!(
                target: "interop_client",
                "Failed to advance derivation pipeline: {:?}",
                e
            );
            Err(e.into())
        }
    }
}

/// Transitions the [PreState] with the given [OptimisticBlock] and checks if the resulting state
/// commitment matches the expected commitment.
fn transition_and_check(
    pre_state: PreState,
    optimistic_block: Option<OptimisticBlock>,
    expected: B256,
) -> Result<(), FaultProofProgramError> {
    let did_append = optimistic_block.is_some();
    let post_state = pre_state
        .transition(optimistic_block)
        .ok_or(FaultProofProgramError::StateTransitionFailed)?;
    let post_state_commitment = post_state.hash();

    if did_append {
        info!(
            target: "interop_client",
            "Appended optimistic L2 block to transition state",
        );
    }

    if post_state_commitment != expected {
        error!(
            target: "interop_client",
            "Failed to validate progressed transition state. Expected post-state commitment: {expected}, actual: {actual}",
            expected = expected,
            actual = post_state_commitment
        );

        return Err(FaultProofProgramError::InvalidClaim(expected, post_state_commitment));
    }

    info!(
        target: "interop_client",
        "Successfully validated progressed transition state with commitment {post_state_commitment}",
    );

    Ok(())
}
