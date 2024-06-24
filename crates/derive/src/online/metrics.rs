//! Metrics for the online derivation pipeline.

use alloc::boxed::Box;
use lazy_static::lazy_static;
use prometheus::{self, register_histogram_vec, HistogramVec};

const RESPONSE_TIME_CUSTOM_BUCKETS: &[f64; 14] =
    &[0.0005, 0.001, 0.002, 0.005, 0.008, 0.01, 0.02, 0.05, 0.08, 0.1, 0.2, 0.5, 0.8, 1.0];

lazy_static! {
    pub static ref PROVIDER_RESPONSE_TIME: HistogramVec = register_histogram_vec!(
        "provider_response_time_seconds",
        "Provider response times",
        &["provider", "method"],
        RESPONSE_TIME_CUSTOM_BUCKETS.to_vec()
    )
    .expect("Failed to register histogram vec");
}
