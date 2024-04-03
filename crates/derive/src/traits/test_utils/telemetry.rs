//! Test Utilities for Telemetry

use crate::traits::{LogLevel, TelemetryProvider};
use alloc::rc::Rc;
use alloc::vec::Vec;
use alloy_primitives::Bytes;
use core::cell::RefCell;

/// Mock telemetry provider
#[derive(Debug, Default)]
pub struct TestTelemetry {
    /// Holds telemetry data with log levels for assertions.
    pub(crate) telemetry_calls: Rc<RefCell<Vec<(Bytes, LogLevel)>>>,
}

impl TestTelemetry {
    /// Creates a new [TestTelemetry] instance.
    pub fn new() -> Self {
        Self::default()
    }
}

impl TelemetryProvider for TestTelemetry {
    fn write<I: Into<alloy_primitives::Bytes>>(&self, data: I, level: LogLevel) {
        let data = (data.into(), level);
        {
            let mut calls = self.telemetry_calls.borrow_mut();
            (*calls).push(data);
        }
    }
}
