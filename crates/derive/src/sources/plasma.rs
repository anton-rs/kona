//! Plasma Data Source

use crate::traits::{AsyncIterator, ChainProvider};
use crate::types::StageResult;
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;

/// A plasma data iterator.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PlasmaSource<CP>
where
    CP: ChainProvider + Send,
{
    /// The chain provider to use for the plasma source.
    chain_provider: CP,
    /// The plasma commitment.
    commitment: Bytes,
    /// The block number.
    block_number: u64,
    /// Whether the plasma source is open.
    open: bool,
}

impl<CP: ChainProvider + Send> PlasmaSource<CP> {
    /// Instantiates a new plasma data source.
    pub fn new(chain_provider: CP) -> Self {
        Self {
            chain_provider,
            commitment: Bytes::default(),
            block_number: 0,
            open: false,
        }
    }
}

#[async_trait]
impl<CP: ChainProvider + Send> AsyncIterator for PlasmaSource<CP> {
    type Item = StageResult<Bytes>;

    async fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
