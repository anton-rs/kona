//! Metrics for derivation pipeline stages.

use alloc::{boxed::Box, string::String};
use lazy_static::lazy_static;
use prometheus::{
    self, opts, register_gauge_vec, register_histogram, register_histogram_vec, register_int_gauge,
    GaugeVec, Histogram, HistogramVec, IntGauge,
};

const RESPONSE_TIME_CUSTOM_BUCKETS: &[f64; 18] = &[
    0.00001, 0.00005, 0.0001, 0.0002, 0.0005, 0.001, 0.002, 0.005, 0.008, 0.01, 0.02, 0.05, 0.08,
    0.1, 0.2, 0.5, 0.8, 1.0,
];

lazy_static! {
    /// Tracks the L1 origin for the L1 Traversal Stage.
    pub static ref ORIGIN_GAUGE: IntGauge = register_int_gauge!(
        "origin_gauge",
        "Tracks the L1 origin for the L1 Traversal Stage"
    ).expect("Origin Gauge failed to register");

    /// Tracks the time taken for provider methods.
    pub static ref PROVIDER_RESPONSE_TIME: HistogramVec = register_histogram_vec!(
        "provider_response_time_seconds",
        "Provider response times",
        &["provider", "method"],
        RESPONSE_TIME_CUSTOM_BUCKETS.to_vec()
    )
    .expect("Failed to register histogram vec");

    /// Tracks the time taken for stage advance methods.
    pub static ref STAGE_ADVANCE_RESPONSE_TIME: HistogramVec = register_histogram_vec!(
        "stage_advance_response_time_seconds",
        "Stage advance response times",
        &["stage"],
        RESPONSE_TIME_CUSTOM_BUCKETS.to_vec()
    ).expect("Failed to register histogram vec");

    /// Tracks the number of derived frames.
    pub static ref DERIVED_FRAMES_COUNT: GaugeVec = {
        let opts = opts!("derived_frames_count", "Number of derived frames");
        register_gauge_vec!(opts, &["status"]).expect("Derived Frames Count failed to register")
    };

    /// Tracks the number of channel timeouts.
    pub static ref CHANNEL_TIMEOUTS: Histogram = {
        let channel_timeout_buckets: [f64; 100] = core::array::from_fn(|i| (i * 10) as f64);
        register_histogram!(
            "channel_timeouts",
            "Channel timeouts",
            channel_timeout_buckets.to_vec()
        ).expect("Failed to register histogram vec")
    };
}
