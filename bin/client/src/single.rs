//! Single-chain fault proof program entrypoint.

use alloc::sync::Arc;
use alloy_consensus::Sealed;
use alloy_primitives::B256;
use core::fmt::Debug;
use kona_driver::{Driver, DriverError};
use kona_executor::{ExecutorError, KonaHandleRegister, TrieDBProvider};
use kona_preimage::{
    CommsClient, HintWriterClient, PreimageKey, PreimageKeyType, PreimageOracleClient,
};
use kona_proof::{
    errors::OracleProviderError,
    executor::KonaExecutor,
    l1::{OracleBlobProvider, OracleL1ChainProvider, OraclePipeline},
    l2::OracleL2ChainProvider,
    sync::new_pipeline_cursor,
    BootInfo, CachingOracle, HintType,
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
    let safe_head_hash = fetch_safe_head_hash(oracle.as_ref(), boot.as_ref()).await?;

    let mut l1_provider = OracleL1ChainProvider::new(boot.l1_head, oracle.clone());
    let mut l2_provider =
        OracleL2ChainProvider::new(safe_head_hash, boot.rollup_config.clone(), oracle.clone());
    let beacon = OracleBlobProvider::new(oracle.clone());

    // Fetch the safe head's block header.
    let safe_head = l2_provider
        .header_by_hash(safe_head_hash)
        .map(|header| Sealed::new_unchecked(header, safe_head_hash))?;

    // If the claimed L2 block number is less than the safe head of the L2 chain, the claim is
    // invalid.
    if boot.claimed_l2_block_number < safe_head.number {
        error!(
            target: "client",
            "Claimed L2 block number {claimed} is less than the safe head {safe}",
            claimed = boot.claimed_l2_block_number,
            safe = safe_head.number
        );
        return Err(FaultProofProgramError::InvalidClaim(
            boot.agreed_l2_output_root,
            boot.claimed_l2_output_root,
        ));
    }

    // In the case where the agreed upon L2 output root is the same as the claimed L2 output root,
    // trace extension is detected and we can skip the derivation and execution steps.
    if boot.agreed_l2_output_root == boot.claimed_l2_output_root {
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
    l2_provider.set_cursor(cursor.clone());

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
    let (number, _, output_root) =
        driver.advance_to_target(&boot.rollup_config, Some(boot.claimed_l2_block_number)).await?;

    ////////////////////////////////////////////////////////////////
    //                          EPILOGUE                          //
    ////////////////////////////////////////////////////////////////

    if output_root != boot.claimed_l2_output_root {
        error!(
            target: "client",
            "Failed to validate L2 block #{number} with output root {output_root}",
            number = number,
            output_root = output_root
        );
        return Err(FaultProofProgramError::InvalidClaim(output_root, boot.claimed_l2_output_root));
    }

    info!(
        target: "client",
        "Successfully validated L2 block #{number} with output root {output_root}",
        number = number,
        output_root = output_root
    );

    Ok(())
}

/// Fetches the safe head hash of the L2 chain based on the agreed upon L2 output root in the
/// [BootInfo].
async fn fetch_safe_head_hash<O>(
    caching_oracle: &O,
    boot_info: &BootInfo,
) -> Result<B256, OracleProviderError>
where
    O: CommsClient,
{
    caching_oracle
        .write(&HintType::StartingL2Output.encode_with(&[boot_info.agreed_l2_output_root.as_ref()]))
        .await
        .map_err(OracleProviderError::Preimage)?;
    let mut output_preimage = [0u8; 128];
    caching_oracle
        .get_exact(
            PreimageKey::new(*boot_info.agreed_l2_output_root, PreimageKeyType::Keccak256),
            &mut output_preimage,
        )
        .await
        .map_err(OracleProviderError::Preimage)?;

    output_preimage[96..128].try_into().map_err(OracleProviderError::SliceConversion)
}
