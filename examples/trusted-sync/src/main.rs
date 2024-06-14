use anyhow::{anyhow, Result};
use clap::Parser;
use kona_derive::{online::*, types::OP_MAINNET_CONFIG};
use reqwest::Url;
use std::sync::Arc;
use tracing::{debug, error, info, warn, Level};

mod cli;
mod validation;

// Environment Variables
const L1_RPC_URL: &str = "L1_RPC_URL";
const L2_RPC_URL: &str = "L2_RPC_URL";
const BEACON_URL: &str = "BEACON_URL";

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = crate::cli::Cli::parse();
    init_tracing_subscriber(cfg.v)?;
    sync(cfg).await
}

async fn sync(cli_cfg: crate::cli::Cli) -> Result<()> {
    // Parse the CLI arguments and environment variables.
    let l1_rpc_url: Url = cli_cfg
        .l1_rpc_url
        .unwrap_or_else(|| std::env::var(L1_RPC_URL).unwrap())
        .parse()
        .expect("valid l1 rpc url");
    let l2_rpc_url: Url = cli_cfg
        .l2_rpc_url
        .unwrap_or_else(|| std::env::var(L2_RPC_URL).unwrap())
        .parse()
        .expect("valid l2 rpc url");
    let beacon_url: String =
        cli_cfg.beacon_url.unwrap_or_else(|| std::env::var(BEACON_URL).unwrap());

    // Construct the pipeline and payload validator.
    let cfg = Arc::new(OP_MAINNET_CONFIG);
    let start = cli_cfg.start_l2_block.unwrap_or(cfg.genesis.l2.number);
    let mut l1_provider = AlloyChainProvider::new_http(l1_rpc_url);
    let mut l2_provider = AlloyL2ChainProvider::new_http(l2_rpc_url.clone(), cfg.clone());
    let attributes =
        StatefulAttributesBuilder::new(cfg.clone(), l2_provider.clone(), l1_provider.clone());
    let beacon_client = OnlineBeaconClient::new_http(beacon_url);
    let blob_provider =
        OnlineBlobProvider::<_, SimpleSlotDerivation>::new(beacon_client, None, None);
    let dap = EthereumDataSource::new(l1_provider.clone(), blob_provider, &cfg);
    let mut cursor = l2_provider
        .l2_block_info_by_number(start)
        .await
        .expect("Failed to fetch genesis L2 block info for pipeline cursor");
    let tip = l1_provider
        .block_info_by_number(cursor.l1_origin.number)
        .await
        .expect("Failed to fetch genesis L1 block info for pipeline tip");
    let validator = validation::OnlineValidator::new_http(
        l2_rpc_url.clone(),
        cfg.canyon_time.unwrap_or_default(),
    );
    let mut pipeline =
        new_online_pipeline(cfg, l1_provider, dap, l2_provider.clone(), attributes, tip);
    let mut derived_attributes_count = 0;

    // Continuously step on the pipeline and validate payloads.
    loop {
        info!(target: "loop", "Validated payload attributes number {}", derived_attributes_count);
        info!(target: "loop", "Pending l2 safe head num: {}", cursor.block_info.number);
        match pipeline.step(cursor).await {
            Ok(_) => info!(target: "loop", "Stepped derivation pipeline"),
            Err(e) => warn!(target: "loop", "Error stepping derivation pipeline: {:?}", e),
        }

        if let Some(attributes) = pipeline.next_attributes() {
            if !validator.validate(&attributes).await {
                error!(target: "loop", "Failed payload validation: {}", attributes.parent.block_info.hash);
                return Ok(());
            }
            derived_attributes_count += 1;
            match l2_provider.l2_block_info_by_number(cursor.block_info.number + 1).await {
                Ok(bi) => cursor = bi,
                Err(e) => {
                    error!(target: "loop", "Failed to fetch next pending l2 safe head: {}, err: {:?}", cursor.block_info.number + 1, e);
                }
            }
            println!(
                "Validated Payload Attributes {derived_attributes_count} [L2 Block Num: {}] [L2 Timestamp: {}] [L1 Origin Block Num: {}]",
                attributes.parent.block_info.number + 1,
                attributes.attributes.timestamp,
                pipeline.origin().unwrap().number,
            );
            info!(target: "loop", "attributes: {:#?}", attributes);
        } else {
            debug!(target: "loop", "No attributes to validate");
        }
    }
}

fn init_tracing_subscriber(v: u8) -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(match v {
            0 => Level::ERROR,
            1 => Level::WARN,
            2 => Level::INFO,
            3 => Level::DEBUG,
            _ => Level::TRACE,
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber).map_err(|e| anyhow!(e))
}
