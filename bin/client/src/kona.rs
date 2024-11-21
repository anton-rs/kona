#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]
#![cfg_attr(any(target_arch = "mips", target_arch = "riscv64"), no_main)]

extern crate alloc;

use alloc::{string::String, sync::Arc};
use kona_driver::{Driver, DriverError};
use kona_preimage::{HintWriter, OracleReader};
use kona_proof::{
    executor::KonaExecutorConstructor,
    l1::{OracleBlobProvider, OracleL1ChainProvider, OraclePipeline},
    l2::OracleL2ChainProvider,
    sync::new_pipeline_cursor,
    BootInfo, CachingOracle,
};
use kona_std_fpvm::{io, FileChannel, FileDescriptor};
use kona_std_fpvm_proc::client_entry;
use tracing::{error, info, warn};

mod handler;
use handler::fpvm_handle_register;

/// The global preimage oracle reader pipe.
static ORACLE_READER_PIPE: FileChannel =
    FileChannel::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite);

/// The global hint writer pipe.
static HINT_WRITER_PIPE: FileChannel =
    FileChannel::new(FileDescriptor::HintRead, FileDescriptor::HintWrite);

/// The global preimage oracle reader.
static ORACLE_READER: OracleReader<FileChannel> = OracleReader::new(ORACLE_READER_PIPE);

/// The global hint writer.
static HINT_WRITER: HintWriter<FileChannel> = HintWriter::new(HINT_WRITER_PIPE);

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

    kona_proof::block_on(async move {
        ////////////////////////////////////////////////////////////////
        //                          PROLOGUE                          //
        ////////////////////////////////////////////////////////////////

        let oracle = Arc::new(CachingOracle::new(ORACLE_LRU_SIZE, ORACLE_READER, HINT_WRITER));
        let boot = match BootInfo::load(oracle.as_ref()).await {
            Ok(boot) => Arc::new(boot),
            Err(e) => {
                error!(target: "client", "Failed to load boot info: {:?}", e);
                io::print(&alloc::format!("Failed to load boot info: {:?}\n", e));
                io::exit(1);
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
            let exit_code =
                if boot.agreed_l2_output_root == boot.claimed_l2_output_root { 0 } else { 1 };
            io::exit(exit_code);
        }

        ////////////////////////////////////////////////////////////////
        //                   DERIVATION & EXECUTION                   //
        ////////////////////////////////////////////////////////////////

        // Create a new derivation driver with the given boot information and oracle.

        let Ok(cursor) = new_pipeline_cursor(
            oracle.clone(),
            &boot,
            &mut l1_provider.clone(),
            &mut l2_provider.clone(),
        )
        .await
        else {
            error!(target: "client", "Failed to find sync start");
            io::print("Failed to find sync start\n");
            io::exit(1);
        };
        let cfg = Arc::new(boot.rollup_config.clone());
        let pipeline = OraclePipeline::new(
            cfg.clone(),
            cursor.clone(),
            oracle.clone(),
            beacon,
            l1_provider.clone(),
            l2_provider.clone(),
        );
        let executor = KonaExecutorConstructor::new(
            &cfg,
            l2_provider.clone(),
            l2_provider,
            fpvm_handle_register,
        );
        let mut driver = Driver::new(cursor, executor, pipeline);

        // Run the derivation pipeline until we are able to produce the output root of the claimed
        // L2 block.
        let (number, output_root) =
            driver.advance_to_target(&boot.rollup_config, boot.claimed_l2_block_number).await?;

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

        Ok::<_, DriverError<kona_executor::ExecutorError>>(())
    })
}
