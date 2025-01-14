//! Multi-chain, interoperable fault proof program entrypoint.

use alloc::{string::ToString, sync::Arc};
use alloy_consensus::Sealed;
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use core::fmt::Debug;
use kona_driver::{Driver, DriverError};
use kona_executor::{ExecutorError, KonaHandleRegister, TrieDBProvider};
use kona_preimage::{
    errors::PreimageOracleError, CommsClient, HintWriterClient, PreimageKey, PreimageKeyType,
    PreimageOracleClient,
};
use kona_proof::{
    errors::OracleProviderError,
    executor::KonaExecutor,
    l1::{OracleBlobProvider, OracleL1ChainProvider, OraclePipeline},
    l2::OracleL2ChainProvider,
    sync::new_pipeline_cursor,
    CachingOracle,
};
use kona_proof_interop::{
    pre_state::{OptimisticBlock, PreState, TransitionState},
    BootInfo, HintType,
};
use thiserror::Error;
use tracing::{error, info};

/// An error that can occur when running the fault proof program.
#[derive(Error, Debug)]
pub enum FaultProofProgramError {
    /// The claim is invalid.
    #[error("Invalid claim. Expected {0}, actual {1}")]
    InvalidClaim(B256, B256),
    /// An error occurred in the Oracle provider.
    #[error(transparent)]
    OracleProviderError(#[from] OracleProviderError),
    /// An error occurred in the driver.
    #[error(transparent)]
    Driver(#[from] DriverError<ExecutorError>),
    /// An error occurred during RLP decoding.
    #[error("RLP decoding error: {0}")]
    RLPDecodingError(alloy_rlp::Error),
}

/// Executes the fault proof program with the given [PreimageOracleClient] and [HintWriterClient].
#[inline]
pub async fn run<P, H>(
    oracle_client: P,
    hint_client: H,
    handle_register: Option<
        KonaHandleRegister<
            OracleL2ChainProvider<CachingOracle<P, H>>,
            OracleL2ChainProvider<CachingOracle<P, H>>,
        >,
    >,
) -> Result<(), FaultProofProgramError>
where
    P: PreimageOracleClient + Send + Sync + Debug + Clone,
    H: HintWriterClient + Send + Sync + Debug + Clone,
{
    const ORACLE_LRU_SIZE: usize = 1024;

    ////////////////////////////////////////////////////////////////
    //                          PROLOGUE                          //
    ////////////////////////////////////////////////////////////////

    let oracle = Arc::new(CachingOracle::new(ORACLE_LRU_SIZE, oracle_client, hint_client));
    let boot = match BootInfo::load(oracle.as_ref()).await {
        Ok(boot) => Arc::new(boot),
        Err(e) => {
            error!(target: "client", "Failed to load boot info: {:?}", e);
            return Err(e.into());
        }
    };

    // Load in the pre-state from the preimage oracle and fetch the L2 safe head block hash.
    let pre =
        PreState::decode(&mut read_raw_pre_state(oracle.as_ref(), boot.as_ref()).await?.as_ref())
            .map_err(FaultProofProgramError::RLPDecodingError)?;
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
    let claimed_l2_block_number = (boot.claimed_l2_timestamp - boot.rollup_config.genesis.l2_time) /
        boot.rollup_config.block_time;

    // If the claimed L2 block number is less than the safe head of the L2 chain, the claim is
    // invalid.
    if claimed_l2_block_number < safe_head.number {
        error!(
            target: "client",
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
            target: "client",
            "Trace extension detected. State transition is already agreed upon.",
        );
        return Ok(());
    }

    ////////////////////////////////////////////////////////////////
    //                   DERIVATION & EXECUTION                   //
    ////////////////////////////////////////////////////////////////

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
    let (number, block_hash, output_root) =
        driver.advance_to_target(&boot.rollup_config, Some(claimed_l2_block_number)).await?;

    ////////////////////////////////////////////////////////////////
    //                          EPILOGUE                          //
    ////////////////////////////////////////////////////////////////

    let optimistic_block = OptimisticBlock::new(block_hash, output_root);
    let transition_state = match pre {
        PreState::SuperRoot(super_root) => {
            TransitionState::new(super_root, alloc::vec![optimistic_block], 1)
        }
        PreState::TransitionState(mut ts) => {
            ts.pending_progress.push(optimistic_block);
            ts.step += 1;
            ts
        }
    };

    if transition_state.hash() != boot.claimed_post_state {
        error!(
            target: "client",
            "Failed to validate L2 block #{number} with output root {output_root}",
            number = number,
            output_root = output_root
        );
        return Err(FaultProofProgramError::InvalidClaim(
            transition_state.hash(),
            boot.claimed_post_state,
        ));
    }

    info!(
        target: "client",
        "Successfully validated L2 block #{number} with output root {output_root}",
        number = number,
        output_root = output_root
    );

    Ok(())
}

/// Reads the raw pre-state from the preimage oracle.
async fn read_raw_pre_state<O>(
    caching_oracle: &O,
    boot_info: &BootInfo,
) -> Result<Bytes, OracleProviderError>
where
    O: CommsClient,
{
    caching_oracle
        .write(&HintType::AgreedPreState.encode_with(&[boot_info.agreed_pre_state.as_ref()]))
        .await
        .map_err(OracleProviderError::Preimage)?;
    let pre = caching_oracle
        .get(PreimageKey::new(*boot_info.agreed_pre_state, PreimageKeyType::Keccak256))
        .await
        .map_err(OracleProviderError::Preimage)?;

    if pre.is_empty() {
        return Err(OracleProviderError::Preimage(PreimageOracleError::Other(
            "Invalid pre-state preimage".to_string(),
        )));
    }

    Ok(Bytes::from(pre))
}

/// Fetches the safe head hash of the L2 chain based on the agreed upon L2 output root in the
/// [BootInfo].
async fn fetch_l2_safe_head_hash<O>(
    caching_oracle: &O,
    pre: &PreState,
) -> Result<B256, OracleProviderError>
where
    O: CommsClient,
{
    // Fetch the output root of the safe head block for the current L2 chain.
    let rich_output = match pre {
        PreState::SuperRoot(super_root) => {
            super_root.output_roots.first().ok_or(OracleProviderError::Preimage(
                PreimageOracleError::Other("No output roots in super root".to_string()),
            ))?
        }
        PreState::TransitionState(transition_state) => {
            transition_state.pre_state.output_roots.get(transition_state.step as usize).ok_or(
                OracleProviderError::Preimage(PreimageOracleError::Other(
                    "No output roots in transition state's pending progress".to_string(),
                )),
            )?
        }
    };

    caching_oracle
        .write(
            &HintType::L2OutputRoot.encode_with(&[rich_output.chain_id.to_be_bytes().as_slice()]),
        )
        .await
        .map_err(OracleProviderError::Preimage)?;
    let output_preimage = caching_oracle
        .get(PreimageKey::new(*rich_output.output_root, PreimageKeyType::Keccak256))
        .await
        .map_err(OracleProviderError::Preimage)?;
    output_preimage[96..128].try_into().map_err(OracleProviderError::SliceConversion)
}
