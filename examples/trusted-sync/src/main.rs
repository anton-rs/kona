use anyhow::Result;
use clap::Parser;
use kona_derive::online::*;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

mod cli;
mod metrics;
mod telemetry;
mod validation;

const METRICS_SERVER_ADDR: &str = "127.0.0.1:9090";
const LOG_TARGET: &str = "trusted-sync";

#[actix_web::main]
async fn main() -> Result<()> {
    let cfg = cli::Cli::parse();
    telemetry::init(cfg.v)?;
    let handle = tokio::spawn(async { sync(cfg).await });
    tokio::select! {
        res = metrics::serve_metrics(METRICS_SERVER_ADDR) => {
            error!(target: LOG_TARGET, "Metrics server failed: {:?}", res);
            return res.map_err(|e| anyhow::anyhow!(e));
        }
        val = handle => {
            error!(target: LOG_TARGET, "Sync failed: {:?}", val);
            anyhow::bail!("Sync failed: {:?}", val);
        }
    }
}

async fn sync(cli: cli::Cli) -> Result<()> {
    // Parse CLI arguments.
    let l1_rpc_url = cli.l1_rpc_url()?;
    let l2_rpc_url = cli.l2_rpc_url()?;
    let beacon_url = cli.beacon_url()?;

    // Query for the L2 Chain ID
    let mut l2_provider =
        AlloyL2ChainProvider::new_http(l2_rpc_url.clone(), Arc::new(Default::default()));
    let l2_chain_id =
        l2_provider.chain_id().await.expect("Failed to fetch chain ID from L2 provider");
    let cfg = RollupConfig::from_l2_chain_id(l2_chain_id)
        .expect("Failed to fetch rollup config from L2 chain ID");
    let cfg = Arc::new(cfg);

    // Construct the pipeline
    let mut l1_provider = AlloyChainProvider::new_http(l1_rpc_url);
    let start = cli.start_l2_block.unwrap_or(cfg.genesis.l2.number);
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
    metrics::SAFE_L2_HEAD.inc_by(cursor.block_info.number);
    let tip = l1_provider
        .block_info_by_number(cursor.l1_origin.number)
        .await
        .expect("Failed to fetch genesis L1 block info for pipeline tip");
    let validator = validation::OnlineValidator::new_http(l2_rpc_url.clone(), &cfg);
    let mut pipeline =
        new_online_pipeline(cfg, l1_provider, dap, l2_provider.clone(), attributes, tip);

    // Continuously step on the pipeline and validate payloads.
    loop {
        info!(target: LOG_TARGET, "Validated payload attributes number {}", metrics::DERIVED_ATTRIBUTES_COUNT.get());
        info!(target: LOG_TARGET, "Pending l2 safe head num: {}", cursor.block_info.number);
        match pipeline.step(cursor).await {
            Ok(_) => info!(target: "loop", "Stepped derivation pipeline"),
            Err(e) => warn!(target: "loop", "Error stepping derivation pipeline: {:?}", e),
        }

        if let Some(attributes) = pipeline.next_attributes() {
            if !validator.validate(&attributes).await {
                error!(target: LOG_TARGET, "Failed payload validation: {}", attributes.parent.block_info.hash);
                return Ok(());
            }
            metrics::DERIVED_ATTRIBUTES_COUNT.inc();
            match l2_provider.l2_block_info_by_number(cursor.block_info.number + 1).await {
                Ok(bi) => {
                    cursor = bi;
                    metrics::SAFE_L2_HEAD.inc();
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to fetch next pending l2 safe head: {}, err: {:?}", cursor.block_info.number + 1, e);
                }
            }
            println!(
                "Validated Payload Attributes {} [L2 Block Num: {}] [L2 Timestamp: {}] [L1 Origin Block Num: {}]",
                metrics::DERIVED_ATTRIBUTES_COUNT.get(),
                attributes.parent.block_info.number + 1,
                attributes.attributes.timestamp,
                pipeline.origin().unwrap().number,
            );
            info!(target: LOG_TARGET, "attributes: {:#?}", attributes);
        } else {
            debug!(target: LOG_TARGET, "No attributes to validate");
        }
    }
}
