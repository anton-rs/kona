use anyhow::Result;
use clap::Parser;
use kona_derive::{online::*, types::StageError};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

mod cli;
mod metrics;
mod telemetry;
mod validation;

const LOG_TARGET: &str = "trusted-sync";

#[actix_web::main]
async fn main() -> Result<()> {
    let cfg = cli::Cli::parse();
    if cfg.loki_metrics {
        let loki_addr = cfg.loki_addr();
        telemetry::init_with_loki(cfg.v, loki_addr)?;
    } else {
        telemetry::init(cfg.v)?;
    }
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
    metrics::CHAIN_ID.inc_by(l2_chain_id);
    let cfg = RollupConfig::from_l2_chain_id(l2_chain_id)
        .expect("Failed to fetch rollup config from L2 chain ID");
    let cfg = Arc::new(cfg);
    metrics::GENESIS_L2_BLOCK.inc_by(cfg.genesis.l2.number);

    // Construct the pipeline
    let mut l1_provider = AlloyChainProvider::new_http(l1_rpc_url);
    let l1_chain_id = l1_provider.chain_id().await?;
    metrics::CONSENSUS_CHAIN_ID.inc_by(l1_chain_id);

    let mut start =
        cli.start_l2_block.filter(|n| *n >= cfg.genesis.l2.number).unwrap_or(cfg.genesis.l2.number);

    // If the start block from tip cli flag is specified, find the latest l2 block number
    // and subtract the specified number of blocks to get the start block number.
    if let Some(blocks) = cli.start_blocks_from_tip {
        start = l2_provider.latest_block_number().await?.saturating_sub(blocks);
        info!(target: LOG_TARGET, "Starting {} blocks from tip at L2 block number: {}", blocks, start);
    }
    metrics::START_L2_BLOCK.inc_by(start);
    println!("Starting from L2 block number: {}", metrics::START_L2_BLOCK.get());

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
    metrics::SAFE_L2_HEAD.set(cursor.block_info.number as i64);
    let tip = l1_provider
        .block_info_by_number(cursor.l1_origin.number)
        .await
        .expect("Failed to fetch genesis L1 block info for pipeline tip");
    let validator = validation::OnlineValidator::new_http(l2_rpc_url.clone(), &cfg);
    let genesis_l2_block_number = cfg.genesis.l2.number;
    let mut pipeline =
        new_online_pipeline(cfg, l1_provider, dap, l2_provider.clone(), attributes, tip);

    // Reset metrics so they can be queried.
    metrics::FAILED_PAYLOAD_DERIVATION.reset();
    metrics::DRIFT_WALKBACK.set(0);
    metrics::DRIFT_WALKBACK_TIMESTAMP.set(0);
    metrics::DERIVED_ATTRIBUTES_COUNT.reset();

    // Continuously step on the pipeline and validate payloads.
    let mut advance_cursor_flag = false;
    loop {
        // Update the reference l2 head.
        match l2_provider.latest_block_number().await {
            Ok(latest) => {
                let prev = metrics::REFERENCE_L2_HEAD.get();
                metrics::REFERENCE_L2_HEAD.set(latest as i64);
                let timestamp = match std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|s| s.as_secs())
                {
                    Ok(time) => time,
                    Err(e) => {
                        error!(target: LOG_TARGET, "Failed to get latest timestamp in seconds: {:?}", e);
                        continue;
                    }
                };

                // Update the timestamp
                if latest as i64 > prev {
                    metrics::LATEST_REF_SAFE_HEAD_UPDATE.set(timestamp as i64);
                }

                // Don't check drift if we're within 10 blocks of origin.
                if cursor.block_info.number - genesis_l2_block_number <= 10 {
                    warn!(target: LOG_TARGET, "Can't walk back further. Cursor: {}, Genesis: {}", cursor.block_info.number, genesis_l2_block_number);
                } else {
                    // Check if we have drift - walk back in case of a re-org.
                    // Wait for at least 500 drift and 5 minutes since the last walkback.
                    let drift = latest as i64 - cursor.block_info.number as i64;
                    if drift > 500 &&
                        timestamp as i64 > metrics::DRIFT_WALKBACK_TIMESTAMP.get() + 300
                    {
                        metrics::DRIFT_WALKBACK.set(cursor.block_info.number as i64);
                        warn!(target: LOG_TARGET, "Detected drift of over {} blocks, walking back", drift);
                        cursor = if let Ok(c) =
                            l2_provider.l2_block_info_by_number(cursor.block_info.number - 10).await
                        {
                            c
                        } else {
                            error!(target: LOG_TARGET, "Failed to get walkback block info by number: {}", cursor.block_info.number - 10);
                            continue;
                        };
                        advance_cursor_flag = false;
                        if let Err(e) = pipeline.reset(cursor.block_info).await {
                            error!(target: LOG_TARGET, "Failed to reset pipeline: {:?}", e);
                        }

                        metrics::DRIFT_WALKBACK_TIMESTAMP.set(timestamp as i64);
                    }
                }
            }
            Err(e) => {
                warn!(target: LOG_TARGET, "Failed to fetch latest reference l2 safe head: {:?}", e);
                continue; // retry the reference fetch.
            }
        }
        if advance_cursor_flag {
            match l2_provider.l2_block_info_by_number(cursor.block_info.number + 1).await {
                Ok(bi) => {
                    cursor = bi;
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
        info!(target: LOG_TARGET, "Stepping on cursor block number: {}", cursor.block_info.number);
        match pipeline.step(cursor).await {
            StepResult::PreparedAttributes => {
                metrics::PIPELINE_STEPS.with_label_values(&["success"]).inc();
                info!(target: "loop", "Prepared attributes");
            }
            StepResult::AdvancedOrigin => {
                metrics::PIPELINE_STEPS.with_label_values(&["origin_advance"]).inc();
                info!(target: "loop", "Advanced origin");
            }
            StepResult::OriginAdvanceErr(e) => {
                metrics::PIPELINE_STEPS.with_label_values(&["origin_advance_failure"]).inc();
                warn!(target: "loop", "Could not advance origin: {:?}", e);
            }
            StepResult::StepFailed(e) => match e {
                StageError::NotEnoughData => {
                    metrics::PIPELINE_STEPS.with_label_values(&["not_enough_data"]).inc();
                    info!(target: "loop", "Not enough data to step derivation pipeline");
                }
                _ => {
                    metrics::PIPELINE_STEPS.with_label_values(&["failure"]).inc();
                    error!(target: "loop", "Error stepping derivation pipeline: {:?}", e);
                }
            },
        }

        // Peek at the next prepared attributes and validate them.
        if let Some(attributes) = pipeline.peek() {
            match validator.validate(attributes).await {
                Ok((true, _)) => info!(target: LOG_TARGET, "Validated payload attributes"),
                Ok((false, expected)) => {
                    error!(target: LOG_TARGET, "Failed payload validation. Derived payload attributes: {:?}, Expected: {:?}", attributes, expected);
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
        let derived = attributes.parent.block_info.number as i64 + 1;
        metrics::SAFE_L2_HEAD.set(derived);
        metrics::DERIVED_ATTRIBUTES_COUNT.inc();
        println!(
            "Validated Payload Attributes {} [L2 Block Num: {}] [L2 Timestamp: {}] [L1 Origin Block Num: {}]",
            metrics::DERIVED_ATTRIBUTES_COUNT.get(),
            derived,
            attributes.attributes.timestamp,
            pipeline.origin().unwrap().number,
        );
        debug!(target: LOG_TARGET, "attributes: {:#?}", attributes);
    }
}
