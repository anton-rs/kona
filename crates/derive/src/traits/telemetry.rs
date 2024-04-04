//! Traits for telemetry.

use alloy_primitives::Bytes;

/// Logging Levels.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug level.
    Debug,
    /// Info level.
    #[default]
    Info,
    /// Warning level.
    Warning,
    /// Error level.
    Error,
}

/// A trait for telemetry providers.
pub trait TelemetryProvider {
    /// Write the telemetry data with LOG_LEVEL.
    fn write<I: Into<Bytes>>(&self, data: I, level: LogLevel);
}
