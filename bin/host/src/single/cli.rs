//! This module contains all CLI-specific code for the single chain entrypoint.

use crate::cli::{cli_styles, parse_b256};
use alloy_primitives::B256;
use anyhow::{anyhow, Result};
use clap::Parser;
use maili_genesis::RollupConfig;
use serde::Serialize;
use std::path::PathBuf;

/// The host binary CLI application arguments.
#[derive(Default, Parser, Serialize, Clone, Debug)]
#[command(styles = cli_styles())]
pub struct SingleChainHostCli {
    /// Hash of the L1 head block. Derivation stops after this block is processed.
    #[clap(long, value_parser = parse_b256, env)]
    pub l1_head: B256,
    /// Hash of the agreed upon safe L2 block committed to by `--agreed-l2-output-root`.
    #[clap(long, visible_alias = "l2-head", value_parser = parse_b256, env)]
    pub agreed_l2_head_hash: B256,
    /// Agreed safe L2 Output Root to start derivation from.
    #[clap(long, visible_alias = "l2-output-root", value_parser = parse_b256, env)]
    pub agreed_l2_output_root: B256,
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
        env
    )]
    pub l2_node_address: Option<String>,
    /// Address of L1 JSON-RPC endpoint to use (eth and debug namespace required)
    #[clap(
        long,
        visible_alias = "l1",
        requires = "l2_node_address",
        requires = "l1_beacon_address",
        env
    )]
    pub l1_node_address: Option<String>,
    /// Address of the L1 Beacon API endpoint to use.
    #[clap(
        long,
        visible_alias = "beacon",
        requires = "l1_node_address",
        requires = "l2_node_address",
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

impl SingleChainHostCli {
    /// Returns `true` if the host is running in offline mode.
    pub const fn is_offline(&self) -> bool {
        self.l1_node_address.is_none() &&
            self.l2_node_address.is_none() &&
            self.l1_beacon_address.is_none()
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
    use crate::single::SingleChainHostCli;
    use alloy_primitives::B256;
    use clap::Parser;

    #[test]
    fn test_flags() {
        let zero_hash_str = &B256::ZERO.to_string();
        let default_flags = [
            "single",
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

            let parsed = SingleChainHostCli::try_parse_from(args);
            assert_eq!(parsed.is_ok(), valid);
        }
    }
}
