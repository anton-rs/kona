/// Metrics trait for `L1Traversal`.
pub trait L1TraversalMetrics: Send + Sync {
    /// Records the block number of the last processed block.
    fn record_block_processed(&self, block_number: u64);
    /// Records system config update.
    fn record_system_config_update(&self);
    /// Records reorg detection.
    fn record_reorg_detected(&self);
    /// Records Holocene activation.
    fn record_holocene_activation(&self);
}
