//! Contains the [L1Retrieval] stage of the derivation pipeline.]

use super::L1Traversal;
use crate::{
    traits::{ChainProvider, DataAvailabilityProvider, DataIter, ResettableStage},
    types::{BlockInfo, StageResult, SystemConfig},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::Bytes;
use anyhow::anyhow;
use async_trait::async_trait;

/// The L1 retrieval stage of the derivation pipeline.
#[derive(Debug)]
pub struct L1Retrieval<DAP, CP>
where
    DAP: DataAvailabilityProvider,
    CP: ChainProvider,
{
    /// The previous stage in the pipeline.
    pub prev: L1Traversal<CP>,
    /// The data availability provider to use for the L1 retrieval stage.
    pub provider: DAP,
    /// The current data iterator.
    data: Option<DAP::DataIter<Bytes>>,
}

impl<DAP, CP> L1Retrieval<DAP, CP>
where
    DAP: DataAvailabilityProvider,
    CP: ChainProvider,
{
    /// Creates a new L1 retrieval stage with the given data availability provider and previous stage.
    pub fn new(prev: L1Traversal<CP>, provider: DAP) -> Self {
        Self {
            prev,
            provider,
            data: None,
        }
    }

    /// Returns the current L1 block in the traversal stage, if it exists.
    pub fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }

    /// Retrieves the next data item from the L1 retrieval stage.
    /// If there is data, it pushes it into the next stage.
    /// If there is no data, it returns an error.
    pub async fn next_data(&mut self) -> StageResult<Bytes> {
        if self.data.is_none() {
            let next = self
                .prev
                .next_l1_block()
                .ok_or_else(|| anyhow!("No block to retrieve data from"))?;
            self.data = Some(
                self.provider
                    .open_data(&next, self.prev.system_config.batcher_addr)
                    .await?,
            );
        }

        // Fetch next data item from the iterator.
        let data = self.data.as_mut().and_then(|d| d.next()).ok_or_else(|| {
            self.data = None;
            anyhow!("No more data to retrieve")
        })?;
        Ok(data)
    }
}

#[async_trait]
impl<DAP, CP> ResettableStage for L1Retrieval<DAP, CP>
where
    DAP: DataAvailabilityProvider + Send,
    CP: ChainProvider + Send,
{
    async fn reset(&mut self, base: BlockInfo, cfg: SystemConfig) -> StageResult<()> {
        self.data = Some(self.provider.open_data(&base, cfg.batcher_addr).await?);
        Ok(())
    }
}
