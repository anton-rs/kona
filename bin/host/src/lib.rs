#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

pub mod cli;
pub use cli::{init_tracing_subscriber, HostCli};

pub mod fetcher;
pub mod kv;
pub mod preimage;
pub mod providers;
pub mod server;
pub mod util;

use anyhow::{bail, Result};
use fetcher::Fetcher;
use kona_preimage::{
    BidirectionalChannel, HintReader, HintWriter, NativeChannel, OracleReader, OracleServer,
};
use kona_std_fpvm::{FileChannel, FileDescriptor};
use kv::KeyValueStore;
use server::PreimageServer;
use std::sync::Arc;
use tokio::{sync::RwLock, task};
use tracing::{debug, error, info};

/// Starts the [PreimageServer] in the primary thread. In this mode, the host program has been
/// invoked by the Fault Proof VM and the client program is running in the parent process.
pub async fn start_server(cfg: HostCli) -> Result<()> {
    let (preimage_pipe, hint_pipe) = (
        FileChannel::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite),
        FileChannel::new(FileDescriptor::HintRead, FileDescriptor::HintWrite),
    );
    let oracle_server = OracleServer::new(preimage_pipe);
    let hint_reader = HintReader::new(hint_pipe);
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
    let server = PreimageServer::new(oracle_server, hint_reader, kv_store, fetcher);
    server.start().await?;
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
    let BidirectionalChannel { host: preimage_host, client: preimage_client } =
        BidirectionalChannel::new()?;
    let BidirectionalChannel { host: hint_host, client: hint_client } =
        BidirectionalChannel::new()?;
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
    let server_task =
        task::spawn(start_native_preimage_server(kv_store, fetcher, hint_host, preimage_host));

    // Start the client program in a separate child process.
    let program_task = task::spawn(kona_client::run(
        OracleReader::new(preimage_client),
        HintWriter::new(hint_client),
    ));

    // Execute both tasks and wait for them to complete.
    info!("Starting preimage server and client program.");
    let client_result;
    tokio::select!(
        r = util::flatten_join_result(server_task) => {
            r?;
            error!(target: "kona_host", "Preimage server exited before client program.");
            bail!("Host program exited before client program.");
        },
        r = util::flatten_join_result(program_task) => {
            client_result = r;
            debug!(target: "kona_host", "Client program has exited with result: {client_result:?}.");
        }
    );
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
