//! Contains the [L1Retrieval] stage of the derivation pipeline.

use super::L1Traversal;
use crate::{
    traits::{
        AsyncIterator, ChainProvider, DataAvailabilityProvider, LogLevel, OriginProvider,
        ResettableStage, TelemetryProvider,
    },
    types::{BlockInfo, StageError, StageResult, SystemConfig},
};
use alloc::boxed::Box;
use anyhow::anyhow;
use async_trait::async_trait;

/// Provides L1 blocks for the [L1Retrieval] stage.
/// This is the previous stage in the pipeline.
#[async_trait]
pub trait L1RetrievalProvider {
    /// Returns the next L1 block to retrieve data for.
    async fn next_l1_block(&mut self) -> StageResult<Option<BlockInfo>>;
}


/// The [L1Retrieval] stage of the derivation pipeline.
/// For each L1 [BlockInfo] pulled from the [L1Traversal] stage,
/// [L1Retrieval] fetches the associated data from a specified
/// [DataAvailabilityProvider]. This data is returned as a generic
/// [DataIter] that can be iterated over.
#[derive(Debug)]
pub struct L1Retrieval<DAP, P, T>
where
    DAP: DataAvailabilityProvider,
    P: L1RetrievalProvider + OriginProvider,
    T: TelemetryProvider,
{
    /// The previous stage in the pipeline.
    pub prev: P,
    /// Telemetry provider for the L1 retrieval stage.
    pub telemetry: T,
    /// The data availability provider to use for the L1 retrieval stage.
    pub provider: DAP,
    /// The current data iterator.
    pub(crate) data: Option<DAP::DataIter>,
}

impl<DAP, P, T> L1Retrieval<DAP, P, T>
where
    DAP: DataAvailabilityProvider,
    P: L1RetrievalProvider + OriginProvider,
    T: TelemetryProvider,
{
    /// Creates a new [L1Retrieval] stage with the previous [L1Traversal]
    /// stage and given [DataAvailabilityProvider].
    pub fn new(prev: L1Traversal<CP, T>, provider: DAP, telemetry: T) -> Self {
        Self { prev, telemetry, provider, data: None }
    }

    /// Retrieves the next data item from the L1 retrieval stage.
    /// If there is data, it pushes it into the next stage.
    /// If there is no data, it returns an error.
    pub async fn next_data(&mut self) -> StageResult<DAP::Item> {
        if self.data.is_none() {
            self.telemetry.write(
                alloc::format!("Retrieving data for block: {:?}", self.prev.block),
                LogLevel::Debug,
            );
            let next = self
                .prev
                .next_l1_block()?
                .ok_or_else(|| anyhow!("No block to retrieve data from"))?;
            self.data =
                Some(self.provider.open_data(&next, self.prev.system_config.batcher_addr).await?);
        }

        let data = self.data.as_mut().expect("Cannot be None").next().await.ok_or(StageError::Eof);
        match data {
            Ok(Ok(data)) => Ok(data),
            Err(StageError::Eof) | Ok(Err(StageError::Eof)) => {
                self.data = None;
                Err(StageError::Eof)
            }
            Ok(Err(e)) | Err(e) => Err(e),
        }
    }
}

impl<DAP, P, T> OriginProvider for L1Retrieval<DAP, P, T>
where
    DAP: DataAvailabilityProvider,
    P: L1RetrievalProvider + OriginProvider,
    T: TelemetryProvider,
{
    fn origin(&self) -> Option<&BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<DAP, P, T> ResettableStage for L1Retrieval<DAP, P, T>
where
    DAP: DataAvailabilityProvider + Send,
    P: L1RetrievalProvider + OriginProvider + Send,
    T: TelemetryProvider + Send,
{
    async fn reset(&mut self, base: BlockInfo, cfg: SystemConfig) -> StageResult<()> {
        self.data = Some(self.provider.open_data(&base, cfg.batcher_addr).await?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        stages::l1_traversal::tests::*,
        traits::test_utils::{TestDAP, TestIter, TestTelemetry},
    };
    use alloc::vec;
    use alloy_primitives::{Address, Bytes};

    #[tokio::test]
    async fn test_l1_retrieval_origin() {
        let traversal = new_populated_test_traversal();
        let dap = TestDAP { results: vec![] };
        let telemetry = TestTelemetry::new();
        let retrieval = L1Retrieval::new(traversal, dap, telemetry);
        let expected = BlockInfo::default();
        assert_eq!(retrieval.origin(), Some(&expected));
    }

    #[tokio::test]
    async fn test_l1_retrieval_next_data() {
        let traversal = new_populated_test_traversal();
        let results = vec![Err(StageError::Eof), Ok(Bytes::default())];
        let dap = TestDAP { results };
        let telemetry = TestTelemetry::new();
        let mut retrieval = L1Retrieval::new(traversal, dap, telemetry);
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
        let telemetry = TestTelemetry::new();
        let traversal = new_test_traversal(vec![], vec![]);
        let dap = TestDAP { results: vec![] };
        let mut retrieval =
            L1Retrieval { prev: traversal, telemetry, provider: dap, data: Some(data) };
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
        let telemetry = TestTelemetry::new();
        let traversal = new_populated_test_traversal();
        let dap = TestDAP { results: vec![] };
        let mut retrieval =
            L1Retrieval { prev: traversal, telemetry, provider: dap, data: Some(data) };
        let data = retrieval.next_data().await.unwrap_err();
        assert_eq!(data, StageError::Eof);
        assert!(retrieval.data.is_none());
    }
}
