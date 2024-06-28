use anyhow::Result;
use clap::Parser;
use kona_derive::online::*;
use std::sync::Arc;
use tracing::{debug, error, info};

mod cli;
mod metrics;
mod telemetry;
mod validation;

const LOG_TARGET: &str = "trusted-sync";

#[actix_web::main]
async fn main() -> Result<()> {
    let cfg = cli::Cli::parse();
    let loki_addr = cfg.loki_addr();
    telemetry::init(cfg.v, loki_addr).await?;
    let addr = cfg.metrics_server_addr();
    let handle = tokio::spawn(async { sync(cfg).await });
    tokio::select! {
        res = metrics::serve_metrics(&addr) => {
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

    // Reset the failed payload derivation metric to 0 so it can be queried.
    metrics::FAILED_PAYLOAD_DERIVATION.reset();

    // Continuously step on the pipeline and validate payloads.
    let mut advance_cursor_flag = false;
    loop {
        info!(target: LOG_TARGET, "Validated payload attributes number {}", metrics::DERIVED_ATTRIBUTES_COUNT.get());
        info!(target: LOG_TARGET, "Pending l2 safe head num: {}", cursor.block_info.number);
        if advance_cursor_flag {
            match l2_provider.l2_block_info_by_number(cursor.block_info.number + 1).await {
                Ok(bi) => {
                    cursor = bi;
                    metrics::SAFE_L2_HEAD.inc();
                    advance_cursor_flag = false;
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to fetch next pending l2 safe head: {}, err: {:?}", cursor.block_info.number + 1, e);
                    // We don't need to step on the pipeline if we failed to fetch the next pending
                    // l2 safe head.
                    continue;
                }
            }
        }
        match pipeline.step(cursor).await {
            Ok(_) => {
                metrics::PIPELINE_STEPS.with_label_values(&["success"]).inc();
                info!(target: "loop", "Stepped derivation pipeline");
            }
            Err(e) => {
                metrics::PIPELINE_STEPS.with_label_values(&["failure"]).inc();
                debug!(target: "loop", "Error stepping derivation pipeline: {:?}", e);
            }
        }

        // Peek at the next prepared attributes and validate them.
        if let Some(attributes) = pipeline.peek() {
            match validator.validate(attributes).await {
                Ok(true) => {
                    info!(target: LOG_TARGET, "Validated payload attributes");
                }
                Ok(false) => {
                    error!(target: LOG_TARGET, "Failed payload validation: {}", attributes.parent.block_info.hash);
                    metrics::FAILED_PAYLOAD_DERIVATION.inc();
                    let _ = pipeline.next(); // Take the attributes and continue
                    continue;
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Failed to validate payload attributes: {:?}", e);
                    // Don't take the next attributes, re-try the current one.
                    continue;
                }
            }
        } else {
            debug!(target: LOG_TARGET, "No attributes to validate");
            continue;
        };

        // Take the next attributes from the pipeline since they're valid.
        let attributes = if let Some(attributes) = pipeline.next() {
            attributes
        } else {
            error!(target: LOG_TARGET, "Must have valid attributes");
            continue;
        };

        // If we validated payload attributes, we should advance the cursor.
        advance_cursor_flag = true;
        metrics::DERIVED_ATTRIBUTES_COUNT.inc();
        println!(
            "Validated Payload Attributes {} [L2 Block Num: {}] [L2 Timestamp: {}] [L1 Origin Block Num: {}]",
            metrics::DERIVED_ATTRIBUTES_COUNT.get(),
            attributes.parent.block_info.number + 1,
            attributes.attributes.timestamp,
            pipeline.origin().unwrap().number,
        );
        debug!(target: LOG_TARGET, "attributes: {:#?}", attributes);
    }
}
