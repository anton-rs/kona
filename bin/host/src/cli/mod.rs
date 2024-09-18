//! This module contains all CLI-specific code for the host binary.

use crate::kv::{
    DiskKeyValueStore, LocalKeyValueStore, MemoryKeyValueStore, SharedKeyValueStore,
    SplitKeyValueStore,
};
use alloy_primitives::B256;
use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser};
use op_alloy_genesis::RollupConfig;
use serde::Serialize;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

mod parser;
pub(crate) use parser::parse_b256;

mod tracing_util;
pub use tracing_util::init_tracing_subscriber;

/// The host binary CLI application arguments.
#[derive(Default, Parser, Serialize, Clone, Debug)]
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
    /// Run the specified client program natively as a separate process detached from the host.
    #[clap(long, conflicts_with = "server", required_unless_present = "server")]
    pub exec: Option<String>,
    /// Run in pre-image server mode without executing any client program. If not provided, the
    /// host will run the client program in the host process.
    #[clap(long, conflicts_with = "exec", required_unless_present = "exec")]
    pub server: bool,
    /// The L2 chain ID of a supported chain. If provided, the host will look for the corresponding
    /// rollup config in the superchain registry.
    #[clap(
        long,
        conflicts_with = "rollup_config_path",
        required_unless_present = "rollup_config_path"
    )]
    pub l2_chain_id: Option<u64>,
    /// Path to rollup config. If provided, the host will use this config instead of attempting to
    /// look up the config in the superchain registry.
    #[clap(long, conflicts_with = "l2_chain_id", required_unless_present = "l2_chain_id")]
    pub rollup_config_path: Option<PathBuf>,
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

    /// Reads the [RollupConfig] from the file system and returns it as a string.
    pub fn read_rollup_config(&self) -> Result<RollupConfig> {
        let path = self.rollup_config_path.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "No rollup config path provided. Please provide a path to the rollup config."
            )
        })?;

        // Read the serialized config from the file system.
        let ser_config = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Error reading RollupConfig file: {e}"))?;

        // Deserialize the config and return it.
        serde_json::from_str(&ser_config)
            .map_err(|e| anyhow!("Error deserializing RollupConfig: {e}"))
    }
}

#[cfg(test)]
mod test {
    use crate::HostCli;
    use alloy_primitives::B256;
    use clap::Parser;

    #[test]
    fn test_exclusive_flags() {
        let zero_hash_str = &B256::ZERO.to_string();
        let default_flags = [
            "host",
            "--l1-head",
            zero_hash_str,
            "--l2-head",
            zero_hash_str,
            "--l2-output-root",
            zero_hash_str,
            "--l2-claim",
            zero_hash_str,
            "--l2-block-number",
            "0",
        ];

        let cases = [
            // valid
            (["--server", "--l2-chain-id", "0"].as_slice(), true),
            (["--server", "--rollup-config-path", "dummy"].as_slice(), true),
            (["--exec", "dummy", "--l2-chain-id", "0"].as_slice(), true),
            (["--exec", "dummy", "--rollup-config-path", "dummy"].as_slice(), true),
            // invalid
            (["--server", "--exec", "dummy", "--l2-chain-id", "0"].as_slice(), false),
            (["--l2-chain-id", "0", "--rollup-config-path", "dummy", "--server"].as_slice(), false),
            (["--server"].as_slice(), false),
            (["--exec", "dummy"].as_slice(), false),
            (["--rollup-config-path", "dummy"].as_slice(), false),
            (["--l2-chain-id", "0"].as_slice(), false),
            ([].as_slice(), false),
        ];

        for (args_ext, valid) in cases.into_iter() {
            let args =
                default_flags.iter().chain(args_ext.into_iter()).cloned().collect::<Vec<_>>();

            let parsed = HostCli::try_parse_from(args);
            assert_eq!(parsed.is_ok(), valid);
        }
    }
}
