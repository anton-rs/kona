/// Metrics for a `ChannelReader`.
pub trait ChannelReaderMetrics: Send + Sync {
    /// Records the number of bytes read from the channel.
    fn record_batch_read(&self);
    /// Records the channel being flushed.
    fn record_channel_flushed(&self);
}
