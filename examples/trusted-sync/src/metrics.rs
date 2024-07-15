//! Metrics for the trusted sync example.

use actix_web::{get, App, HttpServer, Responder};
use anyhow::Result;
use prometheus::{self, opts, Encoder, GaugeVec, IntCounter, IntGauge, TextEncoder};

use lazy_static::lazy_static;
use prometheus::{register_gauge_vec, register_int_counter, register_int_gauge};

lazy_static! {
    /// Tracks the starting L2 block number.
    pub static ref START_L2_BLOCK: IntCounter =
        register_int_counter!("trusted_sync_start_l2_block", "Starting L2 block number").expect("Failed to register start L2 block metric");

    /// Tracks the genesis L2 block number.
    pub static ref GENESIS_L2_BLOCK: IntCounter =
        register_int_counter!("trusted_sync_genesis_l2_block", "Genesis L2 block number").expect("Failed to register genesis L2 block metric");

    /// Tracks the Chain ID currently being synced.
    pub static ref CHAIN_ID: IntCounter =
        register_int_counter!("trusted_sync_chain_id", "Chain ID").expect("Failed to register chain ID metric");

    /// Tracks the Chain ID for the consensus layer.
    pub static ref CONSENSUS_CHAIN_ID: IntCounter =
        register_int_counter!("trusted_sync_consensus_chain_id", "Consensus Chain ID").expect("Failed to register consensus chain ID metric");

    /// Tracks the number of failed payload derivations.
    pub static ref FAILED_PAYLOAD_DERIVATION: IntCounter =
        register_int_counter!("trusted_sync_failed_payload_derivation", "Number of failed payload derivations")
            .expect("Failed to register failed payload derivation metric");

    /// Tracks the number of total payload attributes derived.
    pub static ref DERIVED_ATTRIBUTES_COUNT: IntCounter = register_int_counter!(
        "trusted_sync_derived_attributes_count",
        "Number of total payload attributes derived"
    )
    .expect("Failed to register derived attributes count metric");

    /// Tracks the pending L2 safe head.
    pub static ref SAFE_L2_HEAD: IntGauge =
        register_int_gauge!("trusted_sync_safe_l2_head", "Pending L2 safe head").expect("Failed to register safe L2 head metric");

    /// Tracks the reference l2 head.
    pub static ref REFERENCE_L2_HEAD: IntGauge =
        register_int_gauge!("trusted_sync_reference_l2_head", "Reference L2 head").expect("Failed to register reference L2 head metric");

    /// Tracks the block number when a drift walkback last happened.
    pub static ref DRIFT_WALKBACK: IntGauge =
        register_int_gauge!("trusted_sync_drift_walkback", "Latest drift walkback").expect("Failed to register drift walkback metric");

    /// Tracks the timestamp of the last drift walkback.
    pub static ref DRIFT_WALKBACK_TIMESTAMP: IntGauge =
        register_int_gauge!("trusted_sync_drift_walkback_timestamp", "Timestamp of the last drift walkback").expect("Failed to register drift walkback timestamp metric");

    /// Tracks the latest reference l2 safe head update.
    pub static ref LATEST_REF_SAFE_HEAD_UPDATE: IntGauge = register_int_gauge!(
        "trusted_sync_latest_ref_safe_head_update",
        "Latest reference L2 safe head update"
    )
    .expect("Failed to register latest reference L2 safe head update metric");

    /// Tracks the number of pipeline steps.
    pub static ref PIPELINE_STEPS: GaugeVec = {
        let opts = opts!("trusted_sync_pipeline_steps", "Number of pipeline steps");
        register_gauge_vec!(opts, &["status"]).expect("Failed to register pipeline steps metric")
    };
}

/// Starts the metrics server.
pub async fn serve_metrics(bind: &str) -> Result<()> {
    let _ = HttpServer::new(|| App::new().service(index).service(metrics))
        .bind(bind)
        .map_err(|e| anyhow::anyhow!(e))?
        .run()
        .await;
    Ok(())
}

#[get("/")]
async fn index() -> impl Responder {
    "trusted-sync-metrics-server: visit /metrics to view metrics"
}

#[get("/metrics")]
async fn metrics() -> impl Responder {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        tracing::error!("Failed to encode prometheus metrics: {:?}", e);
    }

    let response = String::from_utf8(buffer.clone()).expect("Failed to convert bytes to string");
    buffer.clear();

    response
}
