use crate::{
    cli::{init_tracing_subscriber, HostCli},
    server::PreimageServer,
};
use anyhow::{anyhow, Result};
use clap::Parser;
use command_fds::{CommandFdExt, FdMapping};
use fetcher::Fetcher;
use futures::FutureExt;
use kona_common::FileDescriptor;
use kona_preimage::{HintReader, OracleServer, PipeHandle};
use kv::KeyValueStore;
use std::{
    io::{stderr, stdin, stdout},
    os::fd::AsFd,
    panic::AssertUnwindSafe,
    sync::Arc,
};
use tokio::{
    process::Command,
    sync::{
        watch::{Receiver, Sender},
        RwLock,
    },
    task,
};
use tracing::{error, info};
use types::NativePipeFiles;

mod cli;
mod fetcher;
mod kv;
mod server;
mod types;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = HostCli::parse();
    init_tracing_subscriber(cfg.v)?;

    if cfg.server {
        start_server(cfg).await?;
    } else {
        start_server_and_native_client(cfg).await?;
    }

    info!("Exiting host program.");
    Ok(())
}

/// Starts the [PreimageServer] in the primary thread. In this mode, the host program has been
/// invoked by the Fault Proof VM and the client program is running in the parent process.
async fn start_server(cfg: HostCli) -> Result<()> {
    let (preimage_pipe, hint_pipe) = (
        PipeHandle::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite),
        PipeHandle::new(FileDescriptor::HintRead, FileDescriptor::HintWrite),
    );
    let oracle_server = OracleServer::new(preimage_pipe);
    let hint_reader = HintReader::new(hint_pipe);

    let kv_store = cfg.construct_kv_store();

    let fetcher = (!cfg.is_offline()).then(|| {
        let l1_provider = util::http_provider(&cfg.l1_node_address.expect("Provider must be set"));
        let l2_provider = util::http_provider(&cfg.l2_node_address.expect("Provider must be set"));
        Arc::new(RwLock::new(Fetcher::new(kv_store.clone(), l1_provider, l2_provider)))
    });

    // Start the server and wait for it to complete.
    info!("Starting preimage server.");
    let server = PreimageServer::new(oracle_server, hint_reader, kv_store, fetcher);
    server.start().await?;
    info!("Preimage server has exited.");

    Ok(())
}

/// Starts the [PreimageServer] and the client program in separate threads. The client program is
/// ran natively in this mode.
async fn start_server_and_native_client(cfg: HostCli) -> Result<()> {
    let (preimage_pipe, hint_pipe, files) = util::create_native_pipes()?;
    let kv_store = cfg.construct_kv_store();

    let fetcher = (!cfg.is_offline()).then(|| {
        let l1_provider =
            util::http_provider(cfg.l1_node_address.as_ref().expect("Provider must be set"));
        let l2_provider =
            util::http_provider(cfg.l2_node_address.as_ref().expect("Provider must be set"));
        Arc::new(RwLock::new(Fetcher::new(kv_store.clone(), l1_provider, l2_provider)))
    });

    // Create a channel to signal the server and the client program to exit.
    let (tx_server, rx_server) = tokio::sync::watch::channel(());
    let (tx_program, rx_program) = (tx_server.clone(), rx_server.clone());

    // Create the server and start it.
    let server_task = task::spawn(start_native_preimage_server(
        kv_store,
        fetcher,
        preimage_pipe,
        hint_pipe,
        tx_server,
        rx_server,
    ));

    // Start the client program in a separate child process.
    let program_task = task::spawn(start_native_client_program(cfg, files, tx_program, rx_program));

    // Execute both tasks and wait for them to complete.
    info!("Starting preimage server and client program.");
    tokio::try_join!(
        util::flatten_join_result(server_task),
        util::flatten_join_result(program_task)
    )
    .map_err(|e| anyhow!(e))?;
    info!("Preimage server and client program have joined.");

    Ok(())
}

/// Starts the preimage server in a separate thread. The client program is ran natively in this
/// mode.
async fn start_native_preimage_server<KV>(
    kv_store: Arc<RwLock<KV>>,
    fetcher: Option<Arc<RwLock<Fetcher<KV>>>>,
    preimage_pipe: PipeHandle,
    hint_pipe: PipeHandle,
    tx: Sender<()>,
    mut rx: Receiver<()>,
) -> Result<()>
where
    KV: KeyValueStore + Send + Sync + ?Sized + 'static,
{
    let oracle_server = OracleServer::new(preimage_pipe);
    let hint_reader = HintReader::new(hint_pipe);

    let server = PreimageServer::new(oracle_server, hint_reader, kv_store, fetcher);

    let server_pair_task = task::spawn(async move {
        AssertUnwindSafe(server.start())
            .catch_unwind()
            .await
            .map_err(|_| {
                error!(target: "preimage_server", "Preimage server panicked");
                anyhow!("Preimage server panicked")
            })?
            .map_err(|e| {
                error!(target: "preimage_server", "Preimage server exited with an error");
                anyhow!("Preimage server exited with an error: {:?}", e)
            })
    });
    let rx_server_task = task::spawn(async move { rx.changed().await });

    // Block the current task until either the client program exits or the server exits.
    tokio::select! {
        _ = rx_server_task => {
            info!(target: "preimage_server", "Received shutdown signal from preimage server task.")
        },
        res = util::flatten_join_result(server_pair_task) => {
            res?;
        }
    }

    // Signal to the client program that the server has exited.
    let _ = tx.send(());

    info!("Preimage server has exited.");
    Ok(())
}

/// Starts the client program in a separate child process. The client program is ran natively in
/// this mode.
///
/// ## Takes
/// - `cfg`: The host configuration.
/// - `files`: The files that are used to communicate with the native client.
/// - `tx`: The sender to signal the preimage server to exit.
/// - `rx`: The receiver to wait for the preimage server to exit.
///
/// ## Returns
/// - `Ok(())` if the client program exits successfully.
/// - `Err(_)` if the client program exits with a non-zero status.
async fn start_native_client_program(
    cfg: HostCli,
    files: NativePipeFiles,
    tx: Sender<()>,
    mut rx: Receiver<()>,
) -> Result<()> {
    // Map the file descriptors to the standard streams and the preimage oracle and hint
    // reader's special file descriptors.
    let mut command = Command::new(cfg.exec);
    command
        .fd_mappings(vec![
            FdMapping { parent_fd: stdin().as_fd().try_clone_to_owned().unwrap(), child_fd: 0 },
            FdMapping { parent_fd: stdout().as_fd().try_clone_to_owned().unwrap(), child_fd: 1 },
            FdMapping { parent_fd: stderr().as_fd().try_clone_to_owned().unwrap(), child_fd: 2 },
            FdMapping { parent_fd: files.hint_writ.into(), child_fd: 3 },
            FdMapping { parent_fd: files.hint_read.into(), child_fd: 4 },
            FdMapping { parent_fd: files.preimage_writ.into(), child_fd: 5 },
            FdMapping { parent_fd: files.preimage_read.into(), child_fd: 6 },
        ])
        .expect("No errors may occur when mapping file descriptors.");

    let exec_task = task::spawn(async move {
        let status = command
            .status()
            .await
            .map_err(|e| {
                error!(target: "client_program", "Failed to execute client program: {:?}", e);
                anyhow!("Failed to execute client program: {:?}", e)
            })?
            .success();
        Ok::<_, anyhow::Error>(status)
    });
    let rx_program_task = task::spawn(async move { rx.changed().await });

    // Block the current task until either the client program exits or the server exits.
    tokio::select! {
        _ = rx_program_task => {
            info!(target: "client_program", "Received shutdown signal from preimage server task.")
        },
        res = util::flatten_join_result(exec_task) => {
            if !(res?) {
                // Signal to the preimage server that the client program has exited.
                let _ = tx.send(());
                error!(target: "client_program", "Client program exited with a non-zero status.");
                return Err(anyhow!("Client program exited with a non-zero status."));
            }
        }
    }

    // Signal to the preimage server that the client program has exited.
    let _ = tx.send(());

    info!(target: "client_program", "Client program has exited.");
    Ok(())
}
