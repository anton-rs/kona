//! Consolidation phase of the interop proof program.

use super::FaultProofProgramError;
use alloc::{sync::Arc, vec::Vec};
use core::fmt::Debug;
use kona_interop::MessageGraph;
use kona_preimage::{HintWriterClient, PreimageOracleClient};
use kona_proof::CachingOracle;
use kona_proof_interop::{BootInfo, OracleInteropProvider, PreState};
use revm::primitives::HashMap;
use tracing::info;

/// Executes the consolidation phase of the interop proof with the given [PreimageOracleClient] and
/// [HintWriterClient].
///
/// This phase is responsible for checking the dependencies between [OptimisticBlock]s in the
/// superchain and ensuring that all dependencies are satisfied.
///
/// [OptimisticBlock]: kona_proof_interop::OptimisticBlock
pub(crate) async fn consolidate_dependencies<P, H>(
    oracle: Arc<CachingOracle<P, H>>,
    boot: BootInfo,
) -> Result<(), FaultProofProgramError>
where
    P: PreimageOracleClient + Send + Sync + Debug + Clone,
    H: HintWriterClient + Send + Sync + Debug + Clone,
{
    let provider = OracleInteropProvider::new(oracle, boot.agreed_pre_state.clone());

    info!(target: "client_interop", "Deriving local-safe headers from prestate");

    // Ensure that the pre-state is a transition state.
    let PreState::TransitionState(ref transition_state) = boot.agreed_pre_state else {
        return Err(FaultProofProgramError::StateTransitionFailed);
    };

    let block_hashes = transition_state
        .pending_progress
        .iter()
        .zip(transition_state.pre_state.output_roots.iter())
        .map(|(optimistic_block, pre_state)| (pre_state.chain_id, optimistic_block.block_hash))
        .collect::<HashMap<_, _>>();

    let mut headers = Vec::with_capacity(block_hashes.len());
    for (chain_id, block_hash) in block_hashes {
        let header = provider.header_by_hash(chain_id, block_hash).await?;
        headers.push((chain_id, header.seal(block_hash)));
    }

    info!(target: "client_interop", "Loaded {} local-safe headers", headers.len());

    // TODO: Re-execution w/ bad blocks. Not complete, we just panic if any deps are invalid atm.
    let graph = MessageGraph::derive(headers.as_slice(), provider).await.unwrap();
    graph.resolve().await.unwrap();

    // Transition to the Super Root at the next timestamp.
    //
    // TODO: This won't work if we replace blocks, `transition` doesn't allow replacement of pending
    // progress just yet.
    let post = boot
        .agreed_pre_state
        .transition(None)
        .ok_or(FaultProofProgramError::StateTransitionFailed)?;
    let post_commitment = post.hash();

    // Ensure that the post-state matches the claimed post-state.
    if post_commitment != boot.claimed_post_state {
        return Err(FaultProofProgramError::InvalidClaim(boot.claimed_post_state, post_commitment));
    }

    Ok(())
}
