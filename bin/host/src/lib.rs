#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

pub mod blobs;
pub mod cli;
pub use cli::{init_tracing_subscriber, HostCli};

pub mod fetcher;
pub mod kv;
pub mod preimage;
pub mod server;

use anyhow::Result;
use fetcher::Fetcher;
use kona_preimage::{
    BidirectionalChannel, HintReader, HintWriter, NativeChannel, OracleReader, OracleServer,
};
use kona_std_fpvm::{FileChannel, FileDescriptor};
use kv::KeyValueStore;
use server::PreimageServer;
use std::sync::Arc;
use tokio::{sync::RwLock, task};
use tracing::info;

/// Starts the [PreimageServer] in the primary thread. In this mode, the host program has been
/// invoked by the Fault Proof VM and the client program is running in the parent process.
pub async fn start_server(cfg: HostCli) -> Result<()> {
    let (preimage_chan, hint_chan) = (
        FileChannel::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite),
        FileChannel::new(FileDescriptor::HintRead, FileDescriptor::HintWrite),
    );
    let oracle_server = OracleServer::new(preimage_chan);
    let hint_reader = HintReader::new(hint_chan);
    let kv_store = cfg.construct_kv_store();
    let fetcher = if !cfg.is_offline() {
        let (l1_provider, blob_provider, l2_provider) = cfg.create_providers().await?;
        Some(Arc::new(RwLock::new(Fetcher::new(
            kv_store.clone(),
            l1_provider,
            blob_provider,
            l2_provider,
            cfg.agreed_l2_head_hash,
        ))))
    } else {
        None
    };

    // Start the server and wait for it to complete.
    info!("Starting preimage server.");
    PreimageServer::new(oracle_server, hint_reader, kv_store, fetcher).start().await?;
    info!("Preimage server has exited.");

    Ok(())
}

/// Starts the [PreimageServer] and the client program in separate threads. The client program is
/// ran natively in this mode.
///
/// ## Takes
/// - `cfg`: The host configuration.
///
/// ## Returns
/// - `Ok(exit_code)` if the client program exits successfully.
/// - `Err(_)` if the client program failed to execute, was killed by a signal, or the host program
///   exited first.
pub async fn start_server_and_native_client(cfg: HostCli) -> Result<i32> {
    let hint_chan = BidirectionalChannel::new()?;
    let preimage_chan = BidirectionalChannel::new()?;
    let kv_store = cfg.construct_kv_store();
    let fetcher = if !cfg.is_offline() {
        let (l1_provider, blob_provider, l2_provider) = cfg.create_providers().await?;
        Some(Arc::new(RwLock::new(Fetcher::new(
            kv_store.clone(),
            l1_provider,
            blob_provider,
            l2_provider,
            cfg.agreed_l2_head_hash,
        ))))
    } else {
        None
    };

    // Create the server and start it.
    let server_task = task::spawn(start_native_preimage_server(
        kv_store,
        fetcher,
        hint_chan.host,
        preimage_chan.host,
    ));

    // Start the client program in a separate child process.
    let program_task = task::spawn(kona_client::run(
        OracleReader::new(preimage_chan.client),
        HintWriter::new(hint_chan.client),
        None,
    ));

    // Execute both tasks and wait for them to complete.
    info!("Starting preimage server and client program.");
    let (_, client_result) = tokio::try_join!(server_task, program_task,)?;
    info!(target: "kona_host", "Preimage server and client program have joined.");

    Ok(client_result.is_err() as i32)
}

/// Starts the preimage server in a separate thread. The client program is ran natively in this
/// mode.
pub async fn start_native_preimage_server<KV>(
    kv_store: Arc<RwLock<KV>>,
    fetcher: Option<Arc<RwLock<Fetcher<KV>>>>,
    hint_chan: NativeChannel,
    preimage_chan: NativeChannel,
) -> Result<()>
where
    KV: KeyValueStore + Send + Sync + ?Sized + 'static,
{
    let hint_reader = HintReader::new(hint_chan);
    let oracle_server = OracleServer::new(preimage_chan);

    PreimageServer::new(oracle_server, hint_reader, kv_store, fetcher).start().await
}
