/// Metrics for a `BatchQueue` stage.
pub trait BatchQueueMetrics: Send + Sync {
    /// Records a batch queued count.
    fn record_batches_queued(&self, count: usize);
    /// Records a batch dropped.
    fn record_batch_dropped(&self);
    /// Records an epoch processed.
    fn record_epoch_advanced(&self, epoch: u64);
}
