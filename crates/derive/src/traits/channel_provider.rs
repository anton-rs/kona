/// Metrics trait for `ChannelProvider`.
pub trait ChannelProviderMetrics: Send + Sync {
    /// Records the number of data items consumed and what type of data was consumed.
    fn record_stage_transition(&self, from: &str, to: &str);
    /// Records the number of data items provided.
    fn record_data_item_provided(&self);
}
