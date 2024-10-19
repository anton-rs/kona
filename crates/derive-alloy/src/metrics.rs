//! Metrics for providers.

use lazy_static::lazy_static;
use prometheus::{self, register_counter_vec, register_histogram_vec, CounterVec, HistogramVec};
use std::{boxed::Box, string::String};

const RESPONSE_TIME_CUSTOM_BUCKETS: &[f64; 18] = &[
    0.00001, 0.00005, 0.0001, 0.0002, 0.0005, 0.001, 0.002, 0.005, 0.008, 0.01, 0.02, 0.05, 0.08,
    0.1, 0.2, 0.5, 0.8, 1.0,
];

lazy_static! {
    /// Tracks the number of provider method calls.
    pub static ref PROVIDER_CALLS: CounterVec = register_counter_vec!(
        "kona_derive_provider_calls",
        "Number of provider method calls",
        &["provider", "method"]
    ).expect("Provider Calls failed to register");

    /// Tracks the number of errors in provider methods.
    pub static ref PROVIDER_ERRORS: CounterVec = register_counter_vec!(
        "kona_derive_provider_errors",
        "Number of provider errors",
        &["provider", "method", "error"]
    ).expect("Provider Errors failed to register");

    /// Tracks the time taken for provider methods.
    pub static ref PROVIDER_RESPONSE_TIME: HistogramVec = register_histogram_vec!(
        "kona_derive_provider_response_time_seconds",
        "Provider response times",
        &["provider", "method"],
        RESPONSE_TIME_CUSTOM_BUCKETS.to_vec()
    )
    .expect("Failed to register histogram vec");
}
