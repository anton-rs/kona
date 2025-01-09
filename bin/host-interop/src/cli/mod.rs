//! This module contains all CLI-specific code for the host binary.

use crate::{
    blobs::OnlineBlobProvider,
    kv::{
        DiskKeyValueStore, LocalKeyValueStore, MemoryKeyValueStore, SharedKeyValueStore,
        SplitKeyValueStore,
    },
};
use alloy_primitives::{Bytes, B256};
use alloy_provider::ReqwestProvider;
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use anyhow::{anyhow, Result};
use clap::{
    builder::styling::{AnsiColor, Color, Style},
    ArgAction, Parser,
};
use op_alloy_genesis::RollupConfig;
use parser::{parse_bytes, parse_key_val};
use reqwest::Client;
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

mod parser;
pub(crate) use parser::parse_b256;

mod tracing_util;
pub use tracing_util::init_tracing_subscriber;

const ABOUT: &str = "
kona-host is a CLI application that runs the Kona pre-image server and client program. The host
can run in two modes: server mode and native mode. In server mode, the host runs the pre-image
server and waits for the client program in the parent process to request pre-images. In native
mode, the host runs the client program in a separate thread with the pre-image server in the
primary thread.
";

/// The host binary CLI application arguments.
#[derive(Default, Parser, Serialize, Clone, Debug)]
#[command(about = ABOUT, version, styles = cli_styles())]
pub struct HostCli {
    /// Verbosity level (0-2)
    #[arg(long, short, action = ArgAction::Count)]
    pub v: u8,
    /// Hash of the L1 head block. Derivation stops after this block is processed.
    #[clap(long, value_parser = parse_b256, env)]
    pub l1_head: B256,
    /// Hash of the agreed upon safe L2 block committed to by `--agreed-l2-output-root`.
    #[clap(long, visible_alias = "l2-head", value_parser = parse_b256, env)]
    pub agreed_l2_head_hash: B256,
    /// Agreed safe L2 Output Root to start derivation from.
    #[clap(long, env, value_parser = parse_bytes)]
    pub agreed_pre_state: Bytes,
    /// Claimed L2 output root at block # `--claimed-l2-block-number` to validate.
    #[clap(long, visible_alias = "l2-claim", value_parser = parse_b256, env)]
    pub claimed_l2_output_root: B256,
    /// Number of the L2 block that the claimed output root commits to.
    #[clap(long, visible_alias = "l2-block-number", env)]
    pub claimed_l2_block_number: u64,
    /// Address of L2 JSON-RPC endpoint to use (eth and debug namespace required).
    #[clap(
        long,
        visible_alias = "l2",
        requires = "l1_node_address",
        requires = "l1_beacon_address",
        env,
        value_parser = parse_key_val::<u64, String>,
        value_delimiter = ','
    )]
    pub l2_node_addresses: Option<Vec<(u64, String)>>,
    /// Address of L1 JSON-RPC endpoint to use (eth and debug namespace required)
    #[clap(
        long,
        visible_alias = "l1",
        requires = "l2_node_addresses",
        requires = "l1_beacon_address",
        env
    )]
    pub l1_node_address: Option<String>,
    /// Address of the L1 Beacon API endpoint to use.
    #[clap(
        long,
        visible_alias = "beacon",
        requires = "l1_node_address",
        requires = "l2_node_addresses",
        env
    )]
    pub l1_beacon_address: Option<String>,
    /// The Data Directory for preimage data storage. Optional if running in online mode,
    /// required if running in offline mode.
    #[clap(
        long,
        visible_alias = "db",
        required_unless_present_all = ["l2_node_address", "l1_node_address", "l1_beacon_address"],
        env
    )]
    pub data_dir: Option<PathBuf>,
    /// Run the client program natively.
    #[clap(long, conflicts_with = "server", required_unless_present = "server")]
    pub native: bool,
    /// Run in pre-image server mode without executing any client program. If not provided, the
    /// host will run the client program in the host process.
    #[clap(long, conflicts_with = "native", required_unless_present = "native")]
    pub server: bool,
    /// The L2 chain ID of a supported chain. If provided, the host will look for the corresponding
    /// rollup config in the superchain registry.
    #[clap(
        long,
        conflicts_with = "rollup_config_path",
        required_unless_present = "rollup_config_path",
        env
    )]
    pub l2_chain_id: Option<u64>,
    /// Path to rollup config. If provided, the host will use this config instead of attempting to
    /// look up the config in the superchain registry.
    #[clap(
        long,
        alias = "rollup-cfg",
        conflicts_with = "l2_chain_id",
        required_unless_present = "l2_chain_id",
        env
    )]
    pub rollup_config_path: Option<PathBuf>,
}

impl HostCli {
    /// Returns `true` if the host is running in offline mode.
    pub const fn is_offline(&self) -> bool {
        self.l1_node_address.is_none()
            && self.l2_node_addresses.is_none()
            && self.l1_beacon_address.is_none()
    }

    /// Returns an HTTP provider for the given URL.
    fn http_provider(url: &str) -> ReqwestProvider {
        let url = url.parse().unwrap();
        let http = Http::<Client>::new(url);
        ReqwestProvider::new(RpcClient::new(http, true))
    }

    /// Creates the providers associated with the [HostCli] configuration.
    ///
    /// ## Returns
    /// - A [ReqwestProvider] for the L1 node.
    /// - An [OnlineBlobProvider] for the L1 beacon node.
    /// - A hash map of chain IDs -> [ReqwestProvider]s for the L2 node.
    pub async fn create_providers(
        &self,
    ) -> Result<(ReqwestProvider, OnlineBlobProvider, HashMap<u64, ReqwestProvider>)> {
        let blob_provider = OnlineBlobProvider::new_http(
            self.l1_beacon_address.clone().ok_or(anyhow!("Beacon API URL must be set"))?,
        )
        .await
        .map_err(|e| anyhow!("Failed to load blob provider configuration: {e}"))?;
        let l1_provider = Self::http_provider(
            self.l1_node_address.as_ref().ok_or(anyhow!("Provider must be set"))?,
        );
        let l2_providers = self
            .l2_node_addresses
            .as_ref()
            .ok_or(anyhow!("L2 node addresses must be set"))?
            .iter()
            .map(|(chain_id, addr)| Ok((*chain_id, Self::http_provider(addr))))
            .collect::<Result<HashMap<_, _>>>()?;

        Ok((l1_provider, blob_provider, l2_providers))
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

/// Styles for the CLI application.
const fn cli_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(Style::new().bold().underline().fg_color(Some(Color::Ansi(AnsiColor::Yellow))))
        .header(Style::new().bold().underline().fg_color(Some(Color::Ansi(AnsiColor::Yellow))))
        .literal(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))))
        .invalid(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .error(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .valid(Style::new().bold().underline().fg_color(Some(Color::Ansi(AnsiColor::Green))))
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::White))))
}

#[cfg(test)]
mod test {
    use crate::HostCli;
    use alloy_primitives::B256;
    use clap::Parser;

    #[test]
    fn test_flags() {
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
            (["--server", "--l2-chain-id", "0", "--data-dir", "dummy"].as_slice(), true),
            (["--server", "--rollup-config-path", "dummy", "--data-dir", "dummy"].as_slice(), true),
            (["--native", "--l2-chain-id", "0", "--data-dir", "dummy"].as_slice(), true),
            (["--native", "--rollup-config-path", "dummy", "--data-dir", "dummy"].as_slice(), true),
            (
                [
                    "--l1-node-address",
                    "dummy",
                    "--l2-node-address",
                    "dummy",
                    "--l1-beacon-address",
                    "dummy",
                    "--server",
                    "--l2-chain-id",
                    "0",
                ]
                .as_slice(),
                true,
            ),
            // invalid
            (["--server", "--native", "--l2-chain-id", "0"].as_slice(), false),
            (["--l2-chain-id", "0", "--rollup-config-path", "dummy", "--server"].as_slice(), false),
            (["--server"].as_slice(), false),
            (["--native"].as_slice(), false),
            (["--rollup-config-path", "dummy"].as_slice(), false),
            (["--l2-chain-id", "0"].as_slice(), false),
            (["--l1-node-address", "dummy", "--server", "--l2-chain-id", "0"].as_slice(), false),
            (["--l2-node-address", "dummy", "--server", "--l2-chain-id", "0"].as_slice(), false),
            (["--l1-beacon-address", "dummy", "--server", "--l2-chain-id", "0"].as_slice(), false),
            ([].as_slice(), false),
        ];

        for (args_ext, valid) in cases.into_iter() {
            let args = default_flags.iter().chain(args_ext.iter()).cloned().collect::<Vec<_>>();

            let parsed = HostCli::try_parse_from(args);
            assert_eq!(parsed.is_ok(), valid);
        }
    }
}
