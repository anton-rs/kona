use crate::{
    cli::{init_tracing_subscriber, HostCli},
    kv::MemoryKeyValueStore,
    server::PreimageServer,
};
use anyhow::{anyhow, Result};
use clap::Parser;
use command_fds::{CommandFdExt, FdMapping};
use fetcher::Fetcher;
use kona_common::FileDescriptor;
use kona_preimage::{HintReader, OracleServer, PipeHandle};
use std::{
    io::{stderr, stdin, stdout},
    os::fd::AsFd,
    process::Command,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::{error, info};

mod cli;
mod fetcher;
mod kv;
mod server;
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

    // TODO: Optional disk store if `cli.data_dir` is set.
    let mem_kv_store = Arc::new(RwLock::new(MemoryKeyValueStore::new()));

    let fetcher = (!cfg.is_offline()).then(|| {
        let l1_provider = util::http_provider(&cfg.l1_node_address.expect("Provider must be set"));
        let l2_provider = util::http_provider(&cfg.l2_node_address.expect("Provider must be set"));
        Arc::new(RwLock::new(Fetcher::new(mem_kv_store.clone(), l1_provider, l2_provider)))
    });

    // Start the server and wait for it to complete.
    info!("Starting preimage server.");
    let server = PreimageServer::new(oracle_server, hint_reader, mem_kv_store, fetcher);
    server.start().await?;
    info!("Preimage server has exited.");

    Ok(())
}

/// Starts the [PreimageServer] and the client program in separate threads. The client program is
/// ran natively in this mode.
async fn start_server_and_native_client(cfg: HostCli) -> Result<()> {
    let (preimage_pipe, hint_pipe, mut files) = util::create_native_pipes()?;
    let oracle_server = OracleServer::new(preimage_pipe);
    let hint_reader = HintReader::new(hint_pipe);

    // TODO: Optional disk store if `cli.data_dir` is set.
    let mem_kv_store = Arc::new(RwLock::new(MemoryKeyValueStore::new()));

    let fetcher = (!cfg.is_offline()).then(|| {
        let l1_provider = util::http_provider(&cfg.l1_node_address.expect("Provider must be set"));
        let l2_provider = util::http_provider(&cfg.l2_node_address.expect("Provider must be set"));
        Arc::new(RwLock::new(Fetcher::new(mem_kv_store.clone(), l1_provider, l2_provider)))
    });

    // Create the server and start it.
    let server = PreimageServer::new(oracle_server, hint_reader, mem_kv_store, fetcher);
    let server_task = tokio::task::spawn(server.start());

    // Start the client program in a separate child process.
    let program_task = tokio::task::spawn(async move {
        let mut command = Command::new(cfg.exec);

        // Map the file descriptors to the standard streams and the preimage oracle and hint
        // reader's special file descriptors.
        command
            .fd_mappings(vec![
                FdMapping { parent_fd: stdin().as_fd().try_clone_to_owned().unwrap(), child_fd: 0 },
                FdMapping {
                    parent_fd: stdout().as_fd().try_clone_to_owned().unwrap(),
                    child_fd: 1,
                },
                FdMapping {
                    parent_fd: stderr().as_fd().try_clone_to_owned().unwrap(),
                    child_fd: 2,
                },
                FdMapping { parent_fd: files.remove(3).into(), child_fd: 3 },
                FdMapping { parent_fd: files.remove(2).into(), child_fd: 4 },
                FdMapping { parent_fd: files.remove(1).into(), child_fd: 5 },
                FdMapping { parent_fd: files.remove(0).into(), child_fd: 6 },
            ])
            .expect("No errors may occur when mapping file descriptors.");

        Ok(command.status().map_err(|e| anyhow!(e))?.success())
    });

    info!("Starting preimage server and client program.");
    let (server_res, program_res) =
        tokio::try_join!(server_task, program_task).map_err(|e| anyhow!(e))?;
    server_res?;
    if !program_res? {
        error!("Client program exited with a non-zero status.");
    }
    info!("Preimage server and client program have exited.");

    Ok(())
}
