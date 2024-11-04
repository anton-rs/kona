#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]
#![cfg_attr(any(target_arch = "mips", target_arch = "riscv64"), no_main)]

extern crate alloc;

use alloc::{string::String, sync::Arc};
use kona_client::{
    errors::DriverError,
    l1::{DerivationDriver, OracleBlobProvider, OracleL1ChainProvider},
    l2::OracleL2ChainProvider,
    BootInfo, CachingOracle,
};
use kona_common::io;
use kona_common_proc::client_entry;

pub(crate) mod fault;
use fault::{fpvm_handle_register, HINT_WRITER, ORACLE_READER};
use tracing::{error, info, warn};

/// The size of the LRU cache in the oracle.
const ORACLE_LRU_SIZE: usize = 1024;

#[client_entry(100_000_000)]
fn main() -> Result<(), String> {
    #[cfg(feature = "tracing-subscriber")]
    {
        use tracing::Level;

        let subscriber = tracing_subscriber::fmt().with_max_level(Level::DEBUG).finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    }

    kona_common::block_on(async move {
        ////////////////////////////////////////////////////////////////
        //                          PROLOGUE                          //
        ////////////////////////////////////////////////////////////////

        let oracle = Arc::new(CachingOracle::new(ORACLE_LRU_SIZE, ORACLE_READER, HINT_WRITER));
        let boot = Arc::new(BootInfo::load(oracle.as_ref()).await?);
        let l1_provider = OracleL1ChainProvider::new(boot.clone(), oracle.clone());
        let l2_provider = OracleL2ChainProvider::new(boot.clone(), oracle.clone());
        let beacon = OracleBlobProvider::new(oracle.clone());

        // If the genesis block is claimed, we can exit early.
        // The agreed upon prestate is consented to by all parties, and there is no state
        // transition, so the claim is valid if the claimed output root matches the agreed
        // upon output root.
        if boot.claimed_l2_block_number == 0 {
            warn!("Genesis block claimed. Exiting early.");
            let exit_code =
                if boot.agreed_l2_output_root == boot.claimed_l2_output_root { 0 } else { 1 };
            io::exit(exit_code);
        }

        ////////////////////////////////////////////////////////////////
        //                   DERIVATION & EXECUTION                   //
        ////////////////////////////////////////////////////////////////

        // Create a new derivation driver with the given boot information and oracle.
        let mut driver =
            DerivationDriver::new(boot.as_ref(), &oracle, beacon, l1_provider, l2_provider.clone())
                .await?;

        // Run the derivation pipeline until we are able to produce the output root of the claimed
        // L2 block.
        let (number, output_root) = driver
            .advance_to_target(
                &boot.rollup_config,
                &l2_provider,
                &l2_provider,
                fpvm_handle_register,
            )
            .await?;

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
            io::print(&alloc::format!(
                "Failed to validate L2 block #{} with output root {}\n",
                number,
                output_root
            ));
            io::exit(1);
        }

        info!(
            target: "client",
            "Successfully validated L2 block #{number} with output root {output_root}",
            number = number,
            output_root = output_root
        );
        io::print(&alloc::format!(
            "Successfully validated L2 block #{} with output root {}\n",
            number,
            output_root
        ));

        Ok::<_, DriverError>(())
    })
}
