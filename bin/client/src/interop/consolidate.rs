//! Consolidation phase of the interop proof program.

use super::FaultProofProgramError;
use alloc::{sync::Arc, vec::Vec};
use core::fmt::Debug;
use kona_interop::MessageGraph;
use kona_preimage::{HintWriterClient, PreimageOracleClient};
use kona_proof::CachingOracle;
use kona_proof_interop::{OracleInteropProvider, PreState};
use revm::primitives::HashMap;

/// Executes the consolidation phase of the interop proof with the given [PreimageOracleClient] and
/// [HintWriterClient].
///
/// This phase is responsible for checking the dependencies between [OptimisticBlock]s in the
/// superchain and ensuring that all dependencies are satisfied.
///
/// [OptimisticBlock]: kona_proof_interop::OptimisticBlock
pub(crate) async fn consolidate_dependencies<P, H>(
    oracle: Arc<CachingOracle<P, H>>,
    pre: PreState,
) -> Result<(), FaultProofProgramError>
where
    P: PreimageOracleClient + Send + Sync + Debug + Clone,
    H: HintWriterClient + Send + Sync + Debug + Clone,
{
    let provider = OracleInteropProvider::new(oracle, pre.clone());

    // Ensure that the pre-state is a transition state.
    let PreState::TransitionState(transition_state) = pre else {
        return Err(FaultProofProgramError::StateTransitionFailed);
    };

    let block_hashes = transition_state
        .pending_progress
        .iter()
        .zip(transition_state.pre_state.output_roots)
        .map(|(optimistic_block, pre_state)| (pre_state.chain_id, optimistic_block.block_hash))
        .collect::<HashMap<_, _>>();

    let mut headers = Vec::with_capacity(block_hashes.len());
    for (chain_id, block_hash) in block_hashes {
        let header = provider.header_by_hash(chain_id, block_hash).await?;
        headers.push((chain_id, header.seal(block_hash)));
    }

    let graph = MessageGraph::derive(headers.as_slice(), provider).await.unwrap();
    graph.resolve().await.unwrap();

    Ok(())
}
