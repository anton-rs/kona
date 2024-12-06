#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]
#![cfg_attr(any(target_arch = "mips", target_arch = "riscv64"), no_main)]

extern crate alloc;

mod evm_config;
pub use evm_config::KonaEvmConfig;

mod precompiles;
use precompiles::fpvm_handle_register;

use alloc::{sync::Arc, string::String};
use kona_std_fpvm_proc::client_entry;
use kona_preimage::{HintWriter, OracleReader};
use kona_std_fpvm::{FileChannel, FileDescriptor};
use reth_optimism_chainspec::OP_MAINNET;

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


#[client_entry(100_000_000)]
fn main() -> Result<(), String> {
    #[cfg(feature = "client-tracing")]
    {
        use kona_std_fpvm::tracing::FpvmTracingSubscriber;

        let subscriber = FpvmTracingSubscriber::new(tracing::Level::INFO);
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    }

    // // ZTODO: derive this from the rollup config
    let evm_config = KonaEvmConfig::new(Arc::new((**OP_MAINNET).clone()));

    kona_proof::block_on(kona_client::run(
        ORACLE_READER,
        HINT_WRITER,
        evm_config
    ))
}
