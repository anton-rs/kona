#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloy_primitives::B256;
use core::fmt::Debug;
use kona_driver::{Driver, DriverError};
use kona_executor::ExecutorError;
use kona_preimage::{HintWriterClient, PreimageOracleClient};
use kona_proof::{
    errors::OracleProviderError,
    executor::KonaExecutorConstructor,
    l1::{OracleBlobProvider, OracleL1ChainProvider, OraclePipeline},
    l2::OracleL2ChainProvider,
    sync::new_pipeline_cursor,
    BootInfo, CachingOracle,
};
use thiserror::Error;
use tracing::{error, info, warn};

mod precompiles;
pub use precompiles::{
    EcPairingAccelerated, EcPairingAcceleratedGranite, EcRecoverAccelerated,
    KZGPointEvalAccelerated, ECPAIRING_ADDRESS, ECRECOVER_ADDRESS, POINT_EVAL_ADDRESS,
};

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
pub async fn run<P, H>(oracle_client: P, hint_client: H) -> Result<(), FaultProofProgramError>
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
    let l1_provider = OracleL1ChainProvider::new(boot.clone(), oracle.clone());
    let l2_provider = OracleL2ChainProvider::new(boot.clone(), oracle.clone());
    let beacon = OracleBlobProvider::new(oracle.clone());

    // If the genesis block is claimed, we can exit early.
    // The agreed upon prestate is consented to by all parties, and there is no state
    // transition, so the claim is valid if the claimed output root matches the agreed
    // upon output root.
    if boot.claimed_l2_block_number == 0 {
        warn!("Genesis block claimed. Exiting early.");
        if boot.agreed_l2_output_root == boot.claimed_l2_output_root {
            info!(
                target: "client",
                "Successfully validated genesis block with output root {output_root}",
                output_root = boot.agreed_l2_output_root
            );
            return Ok(());
        } else {
            error!(
                target: "client",
                "Failed to validate genesis block. Expected {genesis_root}, actual {claimed_root}",
                genesis_root = boot.agreed_l2_output_root,
                claimed_root = boot.claimed_l2_output_root
            );
            return Err(FaultProofProgramError::InvalidClaim(
                boot.agreed_l2_output_root,
                boot.claimed_l2_output_root,
            ));
        };
    }

    ////////////////////////////////////////////////////////////////
    //                   DERIVATION & EXECUTION                   //
    ////////////////////////////////////////////////////////////////

    // Create a new derivation driver with the given boot information and oracle.
    let cursor = new_pipeline_cursor(
        oracle.clone(),
        &boot,
        &mut l1_provider.clone(),
        &mut l2_provider.clone(),
    )
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
    let executor = KonaExecutorConstructor::new(&cfg, l2_provider.clone(), l2_provider, None);
    let mut driver = Driver::new(cursor, executor, pipeline);

    // Run the derivation pipeline until we are able to produce the output root of the claimed
    // L2 block.
    let (number, output_root) =
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
