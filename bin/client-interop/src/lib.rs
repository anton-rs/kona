#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![allow(clippy::type_complexity)]
#![no_std]

extern crate alloc;

use alloc::{string::ToString, sync::Arc};
use alloy_consensus::{Header, Sealed};
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use core::fmt::Debug;
use kona_driver::{Driver, DriverError};
use kona_executor::{ExecutorError, KonaHandleRegister, TrieDBProvider};
use kona_interop::{OutputRootWithBlockHash, SuperRoot, TransitionState};
use kona_preimage::{
    errors::PreimageOracleError, CommsClient, HintWriterClient, PreimageKey, PreimageKeyType,
    PreimageOracleClient,
};
use kona_proof_interop::{
    errors::OracleProviderError,
    executor::KonaExecutor,
    l1::{OracleBlobProvider, OracleL1ChainProvider, OraclePipeline},
    l2::OracleL2ChainProvider,
    sync::new_pipeline_cursor,
    BootInfo, CachingOracle, HintType,
};
use thiserror::Error;
use tracing::{error, info, warn};

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
    let mut l1_provider = OracleL1ChainProvider::new(boot.clone(), oracle.clone());
    let mut l2_provider = OracleL2ChainProvider::new(boot.clone(), oracle.clone());
    let beacon = OracleBlobProvider::new(oracle.clone());

    // If the claimed L2 block number is less than the safe head of the L2 chain, the claim is
    // invalid.
    info!("fetching safe head");
    let safe_head = fetch_safe_head(oracle.as_ref(), boot.as_ref(), &mut l2_provider).await?;
    info!("safe head fetched");

    // Translate the claimed timestamp to an L2 block number.
    let claimed_l2_block_number = (boot.claimed_l2_timestamp - boot.rollup_config.genesis.l2_time)
        / boot.rollup_config.block_time;
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

    // In the case where the agreed upon L2 output root is the same as the claimed L2 output root,
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
    let cursor = new_pipeline_cursor(&boot, safe_head, &mut l1_provider, &mut l2_provider).await?;
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
    let (number, output_root, block_hash) =
        driver.advance_to_target(&boot.rollup_config, Some(claimed_l2_block_number)).await?;
    let output_root_with_hash = OutputRootWithBlockHash::new(block_hash, output_root);

    ////////////////////////////////////////////////////////////////
    //                          EPILOGUE                          //
    ////////////////////////////////////////////////////////////////

    // Check if the pre-state is a TransitionState or SuperRoot.
    let pre = read_raw_pre_state(oracle.as_ref(), boot.as_ref()).await?;
    let transition_state = if pre[0] == kona_interop::SUPER_ROOT_VERSION {
        let super_root =
            SuperRoot::decode(&mut pre[..].as_ref()).map_err(OracleProviderError::Rlp)?;

        TransitionState::new(super_root, alloc::vec![output_root_with_hash], 1)
    } else if pre[0] == kona_interop::TRANSITION_STATE_VERSION {
        let mut transition_state =
            TransitionState::decode(&mut pre[..].as_ref()).map_err(OracleProviderError::Rlp)?;

        transition_state.pending_progress.push(output_root_with_hash);
        transition_state.step += 1;

        transition_state
    } else {
        return Err(OracleProviderError::Preimage(PreimageOracleError::Other(
            "Invalid pre-state version".to_string(),
        ))
        .into());
    };

    if transition_state.hash() != boot.claimed_post_state {
        error!(
            target: "client",
            "Failed to validate L2 block #{number} with claim {output_root}",
            number = number,
            output_root = output_root
        );
        return Err(FaultProofProgramError::InvalidClaim(output_root, transition_state.hash()));
    }

    info!(
        target: "client",
        "Successfully validated L2 block #{number} with output root {output_root}",
        number = number,
        output_root = output_root
    );
    info!(
        target: "client",
        "Transition State: {transition_state:?}",
        transition_state = transition_state
    );

    Ok(())
}

/// Fetches the safe head of the L2 chain based on the agreed upon L2 output root in the
/// [BootInfo].
async fn fetch_safe_head<O>(
    caching_oracle: &O,
    boot_info: &BootInfo,
    l2_chain_provider: &mut OracleL2ChainProvider<O>,
) -> Result<Sealed<Header>, OracleProviderError>
where
    O: CommsClient,
{
    let pre = read_raw_pre_state(caching_oracle, boot_info).await?;

    let safe_hash = if pre[0] == kona_interop::SUPER_ROOT_VERSION {
        let super_root =
            SuperRoot::decode(&mut pre[..].as_ref()).map_err(OracleProviderError::Rlp)?;
        let first_output_root = super_root.output_roots.first().unwrap();

        // Host knows timestamp, host can call `optimsim_outputAtBlock` by converting timestamp to
        // block number.
        caching_oracle
            .write(
                &HintType::L2OutputRoot.encode_with(&[&first_output_root.chain_id.to_be_bytes()]),
            )
            .await
            .map_err(OracleProviderError::Preimage)?;
        let output_preimage = caching_oracle
            .get(PreimageKey::new(*first_output_root.output_root, PreimageKeyType::Keccak256))
            .await
            .map_err(OracleProviderError::Preimage)?;

        output_preimage[96..128].try_into().map_err(OracleProviderError::SliceConversion)?
    } else if pre[0] == kona_interop::TRANSITION_STATE_VERSION {
        // If the pre-state is the transition state, it means that progress on the broader state
        // transition has already begun. We can fetch the last block hash from the pending progress
        // to get the safe head of the .
        let transition_state =
            TransitionState::decode(&mut pre[..].as_ref()).map_err(OracleProviderError::Rlp)?;

        // Find the output root at the current step.
        let rich_output =
            transition_state.pre_state.output_roots.get(transition_state.step as usize).unwrap();

        // Host knows timestamp, host can call `optimsim_outputAtBlock` by converting timestamp to
        // block number.
        caching_oracle
            .write(&HintType::L2OutputRoot.encode_with(&[&rich_output.chain_id.to_be_bytes()]))
            .await
            .map_err(OracleProviderError::Preimage)?;
        let output_preimage = caching_oracle
            .get(PreimageKey::new(*rich_output.output_root, PreimageKeyType::Keccak256))
            .await
            .map_err(OracleProviderError::Preimage)?;

        output_preimage[96..128].try_into().map_err(OracleProviderError::SliceConversion)?
    } else {
        return Err(OracleProviderError::Preimage(PreimageOracleError::Other(
            "Invalid pre-state version".to_string(),
        )));
    };

    l2_chain_provider
        .header_by_hash(safe_hash)
        .map(|header| Sealed::new_unchecked(header, safe_hash))
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
