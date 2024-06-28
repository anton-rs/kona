//! This module contains all CLI-specific code for the host binary.

use crate::kv::{
    DiskKeyValueStore, LocalKeyValueStore, MemoryKeyValueStore, SharedKeyValueStore,
    SplitKeyValueStore,
};
use alloy_primitives::B256;
use clap::{ArgAction, Parser};
use serde::Serialize;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

mod parser;
pub(crate) use parser::parse_b256;

mod tracing_util;
pub(crate) use tracing_util::init_tracing_subscriber;

/// The host binary CLI application arguments.
#[derive(Parser, Serialize, Clone, Debug)]
pub struct HostCli {
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0-4)", action = ArgAction::Count)]
    pub v: u8,
    /// Hash of the L1 head block. Derivation stops after this block is processed.
    #[clap(long, value_parser = parse_b256)]
    pub l1_head: B256,
    /// Hash of the L2 block at the L2 Output Root.
    #[clap(long, value_parser = parse_b256)]
    pub l2_head: B256,
    /// Agreed L2 Output Root to start derivation from.
    #[clap(long, value_parser = parse_b256)]
    pub l2_output_root: B256,
    /// Claimed L2 output root to validate
    #[clap(long, value_parser = parse_b256)]
    pub l2_claim: B256,
    /// Number of the L2 block that the claim is from.
    #[clap(long)]
    pub l2_block_number: u64,
    /// The L2 chain ID.
    #[clap(long)]
    pub l2_chain_id: u64,
    /// Address of L2 JSON-RPC endpoint to use (eth and debug namespace required).
    #[clap(long)]
    pub l2_node_address: Option<String>,
    /// Address of L1 JSON-RPC endpoint to use (eth namespace required)
    #[clap(long)]
    pub l1_node_address: Option<String>,
    /// Address of the L1 Beacon API endpoint to use.
    #[clap(long)]
    pub l1_beacon_address: Option<String>,
    /// The Data Directory for preimage data storage. Default uses in-memory storage.
    #[clap(long)]
    pub data_dir: Option<PathBuf>,
    /// Run the specified client program as a separate process detached from the host. Default is
    /// to run the client program in the host process.
    #[clap(long)]
    pub exec: String,
    /// Run in pre-image server mode without executing any client program. Defaults to `false`.
    #[clap(long)]
    pub server: bool,
}

impl HostCli {
    /// Returns `true` if the host is running in offline mode.
    pub fn is_offline(&self) -> bool {
        self.l1_node_address.is_none() ||
            self.l2_node_address.is_none() ||
            self.l1_beacon_address.is_none()
    }

    /// Parses the CLI arguments and returns a new instance of a [SharedKeyValueStore], as it is
    /// configured to be created.
    pub fn construct_kv_store(&self) -> SharedKeyValueStore {
        let local_kv_store = LocalKeyValueStore::new(self.clone());

        let kv_store: SharedKeyValueStore = if let Some(ref data_dir) = self.data_dir {
            let disk_kv_store = DiskKeyValueStore::new(data_dir.clone());
            let split_kv_store = SplitKeyValueStore::new(local_kv_store, disk_kv_store);
            Arc::new(RwLock::new(split_kv_store))
        } else {
            let mem_kv_store = MemoryKeyValueStore::new();
            let split_kv_store = SplitKeyValueStore::new(local_kv_store, mem_kv_store);
            Arc::new(RwLock::new(split_kv_store))
        };

        kv_store
    }
}
