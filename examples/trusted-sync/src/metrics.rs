//! Metrics for the trusted sync example.

use actix_web::{get, App, HttpServer, Responder};
use anyhow::Result;
use prometheus::{self, Encoder, IntCounter, TextEncoder};

use lazy_static::lazy_static;
use prometheus::register_int_counter;

lazy_static! {
    pub static ref FAILED_PAYLOAD_DERIVATION: IntCounter =
        register_int_counter!("failed_payload_derivation", "Number of failed payload derivations")
            .unwrap();
    pub static ref DERIVED_ATTRIBUTES_COUNT: IntCounter = register_int_counter!(
        "derived_attributes_count",
        "Number of total payload attributes derived"
    )
    .unwrap();
    pub static ref SAFE_L2_HEAD: IntCounter =
        register_int_counter!("safe_l2_head", "Pending L2 safe head").unwrap();
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
