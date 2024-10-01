//! Contains the [L1Retrieval] stage of the derivation pipeline.

use crate::{
    errors::{PipelineError, PipelineErrorKind, PipelineResult},
    stages::FrameQueueProvider,
    traits::{
        AsyncIterator, DataAvailabilityProvider, OriginAdvancer, OriginProvider, PreviousStage,
        ResettableStage,
    },
};
use alloc::boxed::Box;
use alloy_primitives::Address;
use async_trait::async_trait;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::BlockInfo;

/// Provides L1 blocks for the [L1Retrieval] stage.
/// This is the previous stage in the pipeline.
#[async_trait]
pub trait L1RetrievalProvider {
    /// Returns the next L1 [BlockInfo] in the [L1Traversal] stage, if the stage is not complete.
    /// This function can only be called once while the stage is in progress, and will return
    /// [`None`] on subsequent calls unless the stage is reset or complete. If the stage is
    /// complete and the [BlockInfo] has been consumed, an [PipelineError::Eof] error is returned.
    ///
    /// [L1Traversal]: crate::stages::L1Traversal
    async fn next_l1_block(&mut self) -> PipelineResult<Option<BlockInfo>>;

    /// Returns the batcher [Address] from the [op_alloy_genesis::SystemConfig].
    fn batcher_addr(&self) -> Address;
}

/// The [L1Retrieval] stage of the derivation pipeline.
///
/// For each L1 [BlockInfo] pulled from the [L1Traversal] stage, [L1Retrieval] fetches the
/// associated data from a specified [DataAvailabilityProvider]. This data is returned as a generic
/// [DataIter] that can be iterated over.
///
/// [L1Traversal]: crate::stages::L1Traversal
/// [DataIter]: crate::traits::DataAvailabilityProvider::DataIter
#[derive(Debug)]
pub struct L1Retrieval<DAP, P>
where
    DAP: DataAvailabilityProvider,
    P: L1RetrievalProvider + OriginAdvancer + OriginProvider + ResettableStage,
{
    /// The previous stage in the pipeline.
    pub prev: P,
    /// The data availability provider to use for the L1 retrieval stage.
    pub provider: DAP,
    /// The current data iterator.
    pub(crate) data: Option<DAP::DataIter>,
}

impl<DAP, P> L1Retrieval<DAP, P>
where
    DAP: DataAvailabilityProvider,
    P: L1RetrievalProvider + OriginAdvancer + OriginProvider + ResettableStage,
{
    /// Creates a new [L1Retrieval] stage with the previous [L1Traversal] stage and given
    /// [DataAvailabilityProvider].
    ///
    /// [L1Traversal]: crate::stages::L1Traversal
    pub fn new(prev: P, provider: DAP) -> Self {
        crate::set!(STAGE_RESETS, 0, &["l1-retrieval"]);
        Self { prev, provider, data: None }
    }
}

impl<DAP, P> PreviousStage for L1Retrieval<DAP, P>
where
    DAP: DataAvailabilityProvider + Send,
    P: L1RetrievalProvider + OriginAdvancer + OriginProvider + ResettableStage + Send,
{
    type Previous = P;

    fn prev(&self) -> Option<&Self::Previous> {
        Some(&self.prev)
    }

    fn prev_mut(&mut self) -> Option<&mut Self::Previous> {
        Some(&mut self.prev)
    }
}

#[async_trait]
impl<DAP, P> OriginAdvancer for L1Retrieval<DAP, P>
where
    DAP: DataAvailabilityProvider + Send,
    P: L1RetrievalProvider + OriginAdvancer + OriginProvider + ResettableStage + Send,
{
    async fn advance_origin(&mut self) -> PipelineResult<()> {
        self.prev.advance_origin().await
    }
}

#[async_trait]
impl<DAP, P> FrameQueueProvider for L1Retrieval<DAP, P>
where
    DAP: DataAvailabilityProvider + Send,
    P: L1RetrievalProvider + OriginAdvancer + OriginProvider + ResettableStage + Send,
{
    type Item = DAP::Item;

    async fn next_data(&mut self) -> PipelineResult<Self::Item> {
        if self.data.is_none() {
            let next = self
                .prev
                .next_l1_block()
                .await? // SAFETY: This question mark bubbles up the Eof error.
                .ok_or(PipelineError::MissingL1Data.temp())?;
            self.data = Some(self.provider.open_data(&next).await?);
        }

        match self.data.as_mut().expect("Cannot be None").next().await {
            Ok(data) => Ok(data),
            Err(e) => {
                if let PipelineErrorKind::Temporary(PipelineError::Eof) = e {
                    self.data = None;
                }
                Err(e)
            }
        }
    }
}

impl<DAP, P> OriginProvider for L1Retrieval<DAP, P>
where
    DAP: DataAvailabilityProvider + Send,
    P: L1RetrievalProvider + OriginAdvancer + OriginProvider + ResettableStage,
{
    fn origin(&self) -> Option<BlockInfo> {
        self.prev.origin()
    }
}

#[async_trait]
impl<DAP, P> ResettableStage for L1Retrieval<DAP, P>
where
    DAP: DataAvailabilityProvider + Send,
    P: L1RetrievalProvider + OriginAdvancer + OriginProvider + ResettableStage + Send,
{
    async fn reset(&mut self, base: BlockInfo, cfg: &SystemConfig) -> PipelineResult<()> {
        self.prev.reset(base, cfg).await?;
        self.data = Some(self.provider.open_data(&base).await?);
        crate::inc!(STAGE_RESETS, &["l1-retrieval"]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        stages::l1_traversal::tests::*,
        traits::test_utils::{TestDAP, TestIter},
    };
    use alloc::vec;
    use alloy_primitives::Bytes;

    #[tokio::test]
    async fn test_l1_retrieval_origin() {
        let traversal = new_populated_test_traversal();
        let dap = TestDAP { results: vec![], batch_inbox_address: Address::default() };
        let retrieval = L1Retrieval::new(traversal, dap);
        let expected = BlockInfo::default();
        assert_eq!(retrieval.origin(), Some(expected));
    }

    #[tokio::test]
    async fn test_l1_retrieval_next_data() {
        let traversal = new_populated_test_traversal();
        let results = vec![Err(PipelineError::Eof.temp()), Ok(Bytes::default())];
        let dap = TestDAP { results, batch_inbox_address: Address::default() };
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
        assert_eq!(data, PipelineError::Eof.temp());
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
        let traversal = new_test_traversal(vec![], vec![]);
        let dap = TestDAP { results: vec![], batch_inbox_address: Address::default() };
        let mut retrieval = L1Retrieval { prev: traversal, provider: dap, data: Some(data) };
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
            results: vec![Err(PipelineError::Eof.temp())],
        };
        let traversal = new_populated_test_traversal();
        let dap = TestDAP { results: vec![], batch_inbox_address: Address::default() };
        let mut retrieval = L1Retrieval { prev: traversal, provider: dap, data: Some(data) };
        let data = retrieval.next_data().await.unwrap_err();
        assert_eq!(data, PipelineError::Eof.temp());
        assert!(retrieval.data.is_none());
    }
}
