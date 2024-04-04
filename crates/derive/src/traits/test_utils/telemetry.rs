//! Test Utilities for Telemetry

use crate::traits::{LogLevel, TelemetryProvider};
use alloc::{sync::Arc, vec::Vec};
use alloy_primitives::Bytes;
use spin::mutex::Mutex;

/// Mock telemetry provider
#[derive(Debug, Default)]
pub struct TestTelemetry {
    /// Holds telemetry data with log levels for assertions.
    pub(crate) telemetry_calls: Arc<Mutex<Vec<(Bytes, LogLevel)>>>,
}

impl TestTelemetry {
    /// Creates a new [TestTelemetry] instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks the existance of a given ([Bytes], [LogLevel]) call.
    pub fn exists(&self, data: Bytes, level: LogLevel) -> bool {
        let guard = self.telemetry_calls.lock();
        guard.iter().filter(|(d, l)| *d == data && *l == level).count() > 0
    }

    /// Counts the number of telemetry calls with the given [LogLevel].
    pub fn count_calls(&self, level: LogLevel) -> usize {
        let guard = self.telemetry_calls.lock();
        guard.iter().filter(|(_, l)| *l == level).count()
    }
}

impl TelemetryProvider for TestTelemetry {
    fn write<I: Into<alloy_primitives::Bytes>>(&self, data: I, level: LogLevel) {
        let data = (data.into(), level);
        let binding = self.telemetry_calls.clone();
        let mut guard = binding.lock();
        guard.push(data);
    }
}
