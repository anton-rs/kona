//! Macros for use across derive.

/// Starts the timer with a label value.
#[macro_export]
macro_rules! timer {
    (START, $metric:ident, $label:expr, $timer:ident) => {
        #[cfg(feature="metrics")]
        let $timer = $crate::metrics::$metric.with_label_values(&[$label]).start_timer();
        #[cfg(not(feature="metrics"))]
        let $timer = ();
    };
    (DISCARD, $timer:ident) => {
        #[cfg(feature="metrics")]
        $timer.stop_and_discard();
    };
    (STOP, $timer:ident) => {
        #[cfg(feature="metrics")]
        $timer.stop_and_record();
    }
}

/// Increments a metric with a label value.
#[macro_export]
macro_rules! inc_gauge {
    ($metric:ident, $label:expr) => {
        #[cfg(feature="metrics")]
        $crate::metrics::$metric.with_label_values(&[$label]).inc();
    };
    ($metric:ident, $value:expr, $label:expr) => {
        #[cfg(feature="metrics")]
        $crate::metrics::$metric.with_label_values(&[$label]).add($value);
    }
}

/// Observes a metric with a label value.
#[macro_export]
macro_rules! observe_histogram {
    ($metric:ident, $value:expr) => {
        #[cfg(feature="metrics")]
        $crate::metrics::$metric.observe($value);
    };
    ($metric:ident, $value:expr, $label:expr) => {
        #[cfg(feature="metrics")]
        $crate::metrics::$metric.with_label_values(&[$label]).observe($value);
    }
}

/// Sets a metric value.
#[macro_export]
macro_rules! metrics_set {
    ($metric:ident, $value:expr) => {
        #[cfg(feature="metrics")]
        $crate::metrics::$metric.set($value);
    }
}
