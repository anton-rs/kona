//! Contains the [L1Retrieval] stage of the derivation pipeline.]

use super::L1Traversal;
use crate::{
    traits::{ChainProvider, DataAvailabilityProvider, DataIter, ResettableStage},
    types::{BlockInfo, StageError, StageResult, SystemConfig},
};
use alloc::boxed::Box;
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
    pub(crate) data: Option<DAP::DataIter>,
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
                .next_l1_block()?
                .ok_or_else(|| anyhow!("No block to retrieve data from"))?;
            self.data = Some(
                self.provider
                    .open_data(&next, self.prev.system_config.batcher_addr)
                    .await?,
            );
        }

        let data = self.data.as_mut().expect("Cannot be None").next();
        match data {
            Ok(data) => Ok(data),
            Err(StageError::Eof) => {
                self.data = None;
                Err(StageError::Eof)
            }
            Err(e) => Err(e),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stages::l1_traversal::tests::new_test_traversal;
    use crate::traits::test_utils::{TestDAP, TestIter};
    use alloc::vec;
    use alloy_primitives::Address;

    #[tokio::test]
    async fn test_l1_retrieval_origin() {
        let traversal = new_test_traversal(true, true);
        let dap = TestDAP { results: vec![] };
        let retrieval = L1Retrieval::new(traversal, dap);
        let expected = BlockInfo::default();
        assert_eq!(retrieval.origin(), Some(&expected));
    }

    #[tokio::test]
    async fn test_l1_retrieval_next_data() {
        let traversal = new_test_traversal(true, true);
        let results = vec![Err(StageError::Eof), Ok(Bytes::default())];
        let dap = TestDAP { results };
        let mut retrieval = L1Retrieval::new(traversal, dap);
        assert_eq!(retrieval.data, None);
        let data = retrieval.next_data().await.unwrap();
        assert_eq!(data, Bytes::default());
        assert!(retrieval.data.is_some());
        let retrieval_data = retrieval.data.as_ref().unwrap();
        assert_eq!(retrieval_data.open_data_calls.len(), 1);
        assert_eq!(retrieval_data.open_data_calls[0].0, BlockInfo::default());
        assert_eq!(retrieval_data.open_data_calls[0].1, Address::default());
        // Data should be reset to none and the error should be bubbled up.
        let data = retrieval.next_data().await.unwrap_err();
        assert_eq!(data, StageError::Eof);
        assert!(retrieval.data.is_none());
    }

    #[tokio::test]
    async fn test_l1_retrieval_existing_data_is_respected() {
        let data = TestIter {
            open_data_calls: vec![(BlockInfo::default(), Address::default())],
            results: vec![Ok(Bytes::default())],
        };
        // Create a new traversal with no blocks or receipts.
        // This would bubble up an error if the prev stage
        // (traversal) is called in the retrieval stage.
        let traversal = new_test_traversal(false, false);
        let dap = TestDAP { results: vec![] };
        let mut retrieval = L1Retrieval {
            prev: traversal,
            provider: dap,
            data: Some(data),
        };
        let data = retrieval.next_data().await.unwrap();
        assert_eq!(data, Bytes::default());
        assert!(retrieval.data.is_some());
        let retrieval_data = retrieval.data.as_ref().unwrap();
        assert_eq!(retrieval_data.open_data_calls.len(), 1);
    }

    #[tokio::test]
    async fn test_l1_retrieval_existing_data_errors() {
        let data = TestIter {
            open_data_calls: vec![(BlockInfo::default(), Address::default())],
            results: vec![Err(StageError::Eof)],
        };
        let traversal = new_test_traversal(true, true);
        let dap = TestDAP { results: vec![] };
        let mut retrieval = L1Retrieval {
            prev: traversal,
            provider: dap,
            data: Some(data),
        };
        let data = retrieval.next_data().await.unwrap_err();
        assert_eq!(data, StageError::Eof);
        assert!(retrieval.data.is_none());
    }
}
