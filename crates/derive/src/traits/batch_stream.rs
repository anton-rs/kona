/// Metrics for a `BatchStream` stage.
pub trait BatchStreamMetrics: Send + Sync {
    /// Records a batch processed.
    fn record_batch_processed(&self);
    /// Records a span batch accepted.
    fn record_span_batch_accepted(&self);
    /// Records a span batch dropped.
    fn record_span_batch_dropped(&self);
    /// Records the buffer size.
    fn record_buffer_size(&self, size: usize);
}
