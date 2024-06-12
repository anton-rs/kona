use alloy_primitives::{address, b256, U256};
use anyhow::{anyhow, Result};
use kona_derive::{
    online::*,
    types::{BlockID, Genesis, RollupConfig, SystemConfig},
};
use std::sync::Arc;
use tracing::{debug, error, info, warn, Level};

// Environment Variables
const VERBOSITY: &str = "VERBOSITY_LEVEL";
const L1_RPC_URL: &str = "L1_RPC_URL";
const L2_RPC_URL: &str = "L2_RPC_URL";
const BEACON_URL: &str = "BEACON_URL";

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing_subscriber()?;

    // Construct the pipeline and payload validator.
    let cfg = Arc::new(new_op_mainnet_config());
    let l1_provider = AlloyChainProvider::new_http(new_req_url(L1_RPC_URL));
    let mut l2_provider = AlloyL2ChainProvider::new_http(new_req_url(L2_RPC_URL), cfg.clone());
    let attributes =
        StatefulAttributesBuilder::new(cfg.clone(), l2_provider.clone(), l1_provider.clone());
    let beacon_client = OnlineBeaconClient::new_http(new_req_url(BEACON_URL));
    let blob_provider =
        OnlineBlobProvider::<_, SimpleSlotDerivation>::new(true, beacon_client, None, None);
    let dap = EthereumDataSource::new(l1_provider.clone(), blob_provider, &cfg);
    let mut pipeline =
        new_online_pipeline(cfg, l1_provider, dap, l2_provider.clone(), attributes).await;
    let validator = OnlineValidator::new_http(new_req_url(L2_RPC_URL));
    let mut derived_attributes_count = 0;

    // Continuously step on the pipeline and validate payloads.
    loop {
        match pipeline.step().await {
            Ok(_) => info!(target: "loop", "Stepped derivation pipeline"),
            Err(e) => warn!(target: "loop", "Error stepping derivation pipeline: {:?}", e),
        }

        if let Some(attributes) = pipeline.pop() {
            if !validator.validate(&attributes).await {
                error!(target: "loop", "Failed payload validation: {}", attributes.parent.block_info.hash);
                continue;
            }
            derived_attributes_count += 1;
            info!(target: "loop", "Validated payload attributes number {}", derived_attributes_count);
            match l2_provider.l2_block_info_by_number(pipeline.cursor.block_info.number + 1).await {
                Ok(bi) => pipeline.update_cursor(bi),
                Err(e) => {
                    error!(target: "loop", "Failed to fetch next pending l2 safe head: {}, err: {:?}", pipeline.cursor.block_info.number + 1, e);
                }
            }
            dbg!(attributes);
        } else {
            debug!(target: "loop", "No attributes to validate");
        }
    }
}

fn init_tracing_subscriber() -> Result<()> {
    let verbosity_level = std::env::var(VERBOSITY)
        .unwrap_or_else(|_| "3".to_string())
        .parse::<u8>()
        .map_err(|e| anyhow!(e))?;
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(match verbosity_level {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            _ => Level::TRACE,
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber).map_err(|e| anyhow!(e))
}

fn new_req_url(var: &str) -> reqwest::Url {
    std::env::var(var).unwrap_or_else(|_| panic!("{var} must be set")).parse().unwrap()
}

fn new_op_mainnet_config() -> RollupConfig {
    RollupConfig {
        genesis: Genesis {
            l1: BlockID {
                hash: b256!("438335a20d98863a4c0c97999eb2481921ccd28553eac6f913af7c12aec04108"),
                number: 17_422_590_u64,
            },
            l2: BlockID {
                hash: b256!("dbf6a80fef073de06add9b0d14026d6e5a86c85f6d102c36d3d8e9cf89c2afd3"),
                number: 105_235_063_u64,
            },
            timestamp: 1_686_068_903_u64,
            system_config: SystemConfig {
                batcher_addr: address!("6887246668a3b87f54deb3b94ba47a6f63f32985"),
                l1_fee_overhead: U256::from(0xbc),
                l1_fee_scalar: U256::from(0xa6fe0),
                gas_limit: U256::from(30_000_000_u64),
            },
        },
        block_time: 2_u64,
        max_sequencer_drift: 600_u64,
        seq_window_size: 3600_u64,
        channel_timeout: 300_u64,
        l1_chain_id: 1_u64,
        l2_chain_id: 10_u64,
        regolith_time: Some(0_u64),
        canyon_time: Some(1_704_992_401_u64),
        delta_time: Some(1_708_560_000_u64),
        ecotone_time: Some(1_710_374_401_u64),
        fjord_time: Some(1_720_627_201_u64),
        interop_time: None,
        batch_inbox_address: address!("ff00000000000000000000000000000000000010"),
        deposit_contract_address: address!("beb5fc579115071764c7423a4f12edde41f106ed"),
        l1_system_config_address: address!("229047fed2591dbec1ef1118d64f7af3db9eb290"),
        protocol_versions_address: address!("8062abc286f5e7d9428a0ccb9abd71e50d93b935"),
        da_challenge_address: Some(address!("0000000000000000000000000000000000000000")),
        blobs_enabled_l1_timestamp: None,
    }
}
