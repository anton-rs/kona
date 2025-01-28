//! Consolidation phase of the interop proof program.

use super::FaultProofProgramError;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_consensus::{Header, Sealed};
use alloy_primitives::{Bytes, Sealable};
use alloy_rpc_types_engine::PayloadAttributes;
use core::fmt::Debug;
use kona_executor::{NoopTrieDBProvider, StatelessL2BlockExecutor};
use kona_interop::{InteropProvider, MessageGraph, MessageGraphError};
use kona_mpt::NoopTrieHinter;
use kona_preimage::{CommsClient, HintWriterClient, PreimageOracleClient};
use kona_proof::{errors::OracleProviderError, CachingOracle};
use kona_proof_interop::{
    BootInfo, OptimisticBlock, OracleInteropProvider, PreState, SuperchainConsolidator,
};
use maili_genesis::RollupConfig;
use op_alloy_consensus::OpTxType;
use op_alloy_rpc_types_engine::OpPayloadAttributes;
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
    mut pre: PreState,
) -> Result<(), FaultProofProgramError>
where
    P: PreimageOracleClient + Send + Sync + Debug + Clone,
    H: HintWriterClient + Send + Sync + Debug + Clone,
{
    let provider = OracleInteropProvider::new(oracle, pre.clone());

    info!(target: "client_interop", "Deriving local-safe headers from prestate");

    // Ensure that the pre-state is a transition state.
    let PreState::TransitionState(ref transition_state) = pre else {
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

    // Consolidate the superchain, checking dependency validity and recursively re-executing blocks with only
    // their deposit transactions until all blocks in the superchain contain only valid messages.
    //
    // As blocks are re-executed, the pre-state is updated with the new output roots.
    let mut consolidator = SuperchainConsolidator::new(&mut pre, provider, headers);
    // consolidator.consolidate().await?;

    // Transition to the Super Root at the next timestamp.
    //
    // TODO: This won't work if we replace blocks, `transition` doesn't allow replacement of pending
    // progress just yet.
    let post = pre.transition(None).ok_or(FaultProofProgramError::StateTransitionFailed)?;
    let post_commitment = post.hash();

    // Ensure that the post-state matches the claimed post-state.
    if post_commitment != boot.claimed_post_state {
        return Err(FaultProofProgramError::InvalidClaim(boot.claimed_post_state, post_commitment));
    }

    Ok(())
}
