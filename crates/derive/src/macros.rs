//! Macros for use across derive.

/// Starts the timer with a label value.
#[macro_export]
macro_rules! timer {
    (START, $metric:ident, $labels:expr, $timer:ident) => {
        #[cfg(feature = "metrics")]
        let $timer = $crate::metrics::$metric.with_label_values($labels).start_timer();
    };
    (DISCARD, $timer:ident) => {
        #[cfg(feature = "metrics")]
        $timer.stop_and_discard();
    };
    (STOP, $timer:ident) => {
        #[cfg(feature = "metrics")]
        $timer.stop_and_record();
    };
}

/// Increments a metric with a label value.
#[macro_export]
macro_rules! inc {
    ($metric:ident) => {
        #[cfg(feature = "metrics")]
        $crate::metrics::$metric.inc();
    };
    ($metric:ident, $labels:expr) => {
        #[cfg(feature = "metrics")]
        $crate::metrics::$metric.with_label_values($labels).inc();
    };
    ($metric:ident, $value:expr, $labels:expr) => {
        #[cfg(feature = "metrics")]
        $crate::metrics::$metric.with_label_values($labels).add($value);
    };
}

/// Observes a metric with a label value.
#[macro_export]
macro_rules! observe {
    ($metric:ident, $value:expr) => {
        #[cfg(feature = "metrics")]
        $crate::metrics::$metric.observe($value);
    };
    ($metric:ident, $value:expr, $labels:expr) => {
        #[cfg(feature = "metrics")]
        $crate::metrics::$metric.with_label_values($label).observe($value);
    };
}

/// Sets a metric value.
#[macro_export]
macro_rules! set {
    ($metric:ident, $value:expr) => {
        #[cfg(feature = "metrics")]
        $crate::metrics::$metric.set($value);
    };
    ($metric:ident, $value:expr, $labels:expr) => {
        #[cfg(feature = "metrics")]
        $crate::metrics::$metric.with_label_values($labels).set($value as f64);
    };
}
