//! Plasma Data Source

use crate::{
    traits::PlasmaInputFetcher,
    types::{
        decode_keccak256, Keccak256Commitment, PlasmaError, MAX_INPUT_SIZE, TX_DATA_VERSION_1,
    },
};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use anyhow::anyhow;
use async_trait::async_trait;
use kona_derive::{
    traits::{AsyncIterator, ChainProvider},
    types::{BlockID, ResetError, StageError, StageResult},
};

/// A plasma data iterator.
#[derive(Debug, Clone)]
pub struct PlasmaSource<C, F, I>
where
    C: ChainProvider + Send,
    F: PlasmaInputFetcher<C> + Send,
    I: Iterator<Item = Bytes>,
{
    /// The chain provider to use for the plasma source.
    pub chain_provider: C,
    /// The plasma input fetcher.
    pub input_fetcher: F,
    /// A source data iterator.
    pub source: I,
    /// Keeps track of a pending commitment so we can keep trying to fetch the input.
    pub commitment: Option<Keccak256Commitment>,
    /// The block id.
    pub id: BlockID,
}

impl<C, F, I> PlasmaSource<C, F, I>
where
    C: ChainProvider + Send,
    F: PlasmaInputFetcher<C> + Send,
    I: Iterator<Item = Bytes>,
{
    /// Instantiates a new plasma data source.
    pub fn new(chain_provider: C, input_fetcher: F, source: I, id: BlockID) -> Self {
        Self { chain_provider, input_fetcher, source, id, commitment: None }
    }
}

#[async_trait]
impl<C, F, I> AsyncIterator for PlasmaSource<C, F, I>
where
    C: ChainProvider + Send,
    F: PlasmaInputFetcher<C> + Send,
    I: Iterator<Item = Bytes> + Send,
{
    type Item = Bytes;

    async fn next(&mut self) -> StageResult<Self::Item> {
        // Process origin syncs the challenge contract events and updates the local challenge states
        // before we can proceed to fetch the input data. This function can be called multiple times
        // for the same origin and noop if the origin was already processed. It is also called if
        // there is not commitment in the current origin.
        match self.input_fetcher.advance_l1_origin(&self.chain_provider, self.id).await {
            Some(Ok(_)) => {
                tracing::debug!("plasma input fetcher - l1 origin advanced");
            }
            Some(Err(PlasmaError::ReorgRequired)) => {
                tracing::error!("new expired challenge");
                return StageResult::Err(StageError::Reset(ResetError::NewExpiredChallenge));
            }
            Some(Err(e)) => {
                tracing::error!("failed to advance plasma L1 origin: {:?}", e);
                return StageResult::Err(StageError::Temporary(anyhow::anyhow!(
                    "failed to advance plasma L1 origin: {:?}",
                    e
                )));
            }
            None => {
                tracing::warn!("l1 origin advance returned None");
            }
        }

        // Set the commitment if it isn't available.
        if self.commitment.is_none() {
            // The l1 source returns the input commitment for the batch.
            let data = match self.source.next().ok_or(PlasmaError::NotEnoughData) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("failed to pull next data from the plasma source iterator");
                    return Err(StageError::Custom(anyhow!(e)));
                }
            };

            // If the data is empty,
            if data.is_empty() {
                tracing::warn!("empty data from plasma source");
                return Err(StageError::Custom(anyhow!(PlasmaError::NotEnoughData)));
            }

            // If the tx data type is not plasma, we forward it downstream to let the next
            // steps validate and potentially parse it as L1 DA inputs.
            if data[0] != TX_DATA_VERSION_1 {
                tracing::info!("non-plasma tx data, forwarding downstream");
                return Ok(data);
            }

            // Validate that the batcher inbox data is a commitment.
            self.commitment = match decode_keccak256(&data[1..]) {
                Ok(c) => Some(c),
                Err(e) => {
                    tracing::warn!("invalid commitment: {}, err: {}", data, e);
                    return self.next().await;
                }
            };
        }

        // Use the commitment to fetch the input from the plasma DA provider.
        let commitment = self.commitment.as_ref().expect("the commitment must be set");

        // Fetch the input data from the plasma DA provider.
        let data = match self
            .input_fetcher
            .get_input(&self.chain_provider, commitment.clone(), self.id)
            .await
        {
            Some(Ok(data)) => data,
            Some(Err(PlasmaError::ReorgRequired)) => {
                // The plasma fetcher may call for a reorg if the pipeline is stalled and the plasma
                // DA manager continued syncing origins detached from the pipeline
                // origin.
                tracing::warn!("challenge for a new previously derived commitment expired");
                return Err(StageError::Reset(ResetError::ReorgRequired));
            }
            Some(Err(PlasmaError::ChallengeExpired)) => {
                // This commitment was challenged and the challenge expired.
                tracing::warn!("challenge expired, skipping batch");
                self.commitment = None;
                // Skip the input.
                return self.next().await;
            }
            Some(Err(PlasmaError::MissingPastWindow)) => {
                tracing::warn!("missing past window, skipping batch");
                return Err(StageError::Critical(anyhow::anyhow!(
                    "data for commitment {:?} not available",
                    commitment
                )));
            }
            Some(Err(PlasmaError::ChallengePending)) => {
                // Continue stepping without slowing down.
                tracing::debug!("plasma challenge pending, proceeding");
                return Err(StageError::NotEnoughData);
            }
            Some(Err(e)) => {
                // Return temporary error so we can keep retrying.
                return Err(StageError::Temporary(anyhow::anyhow!(
                    "failed to fetch input data with comm {:?} from da service: {:?}",
                    commitment,
                    e
                )));
            }
            None => {
                // Return temporary error so we can keep retrying.
                return Err(StageError::Temporary(anyhow::anyhow!(
                    "failed to fetch input data with comm {:?} from da service",
                    commitment
                )));
            }
        };

        // The data length is limited to a max size to ensure they can be challenged in the DA
        // contract.
        if data.len() > MAX_INPUT_SIZE {
            tracing::warn!("input data (len {}) exceeds max size {MAX_INPUT_SIZE}", data.len());
            self.commitment = None;
            return self.next().await;
        }

        // Reset the commitment so we can fetch the next one from the source at the next iteration.
        self.commitment = None;

        return Ok(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestPlasmaInputFetcher;
    use alloc::vec;
    use kona_derive::{
        stages::test_utils::{CollectingLayer, TraceStorage},
        traits::test_utils::TestChainProvider,
    };
    use tracing::Level;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    #[tokio::test]
    async fn test_next_plasma_advance_origin_reorg_error() {
        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher {
            advances: vec![Err(PlasmaError::ReorgRequired)],
            ..Default::default()
        };
        let source = vec![Bytes::from("hello"), Bytes::from("world")].into_iter();
        let id = BlockID { number: 1, ..Default::default() };

        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap_err();
        assert_eq!(err, StageError::Reset(ResetError::NewExpiredChallenge));
    }

    #[tokio::test]
    async fn test_next_plasma_advance_origin_other_error() {
        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher {
            advances: vec![Err(PlasmaError::NotEnoughData)],
            ..Default::default()
        };
        let source = vec![Bytes::from("hello"), Bytes::from("world")].into_iter();
        let id = BlockID { number: 1, ..Default::default() };

        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap_err();
        matches!(err, StageError::Temporary(_));
    }

    #[tokio::test]
    async fn test_next_plasma_not_enough_source_data() {
        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher { advances: vec![Ok(())], ..Default::default() };
        let source = vec![].into_iter();
        let id = BlockID { number: 1, ..Default::default() };

        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap_err();
        assert_eq!(err, StageError::Custom(anyhow!(PlasmaError::NotEnoughData)));
    }

    #[tokio::test]
    async fn test_next_plasma_empty_source_data() {
        let trace_store: TraceStorage = Default::default();
        let layer = CollectingLayer::new(trace_store.clone());
        tracing_subscriber::Registry::default().with(layer).init();

        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher { advances: vec![Ok(())], ..Default::default() };
        let source = vec![Bytes::from("")].into_iter();
        let id = BlockID { number: 1, ..Default::default() };

        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap_err();
        assert_eq!(err, StageError::Custom(anyhow!(PlasmaError::NotEnoughData)));

        let logs = trace_store.get_by_level(Level::WARN);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("empty data from plasma source"));
    }

    #[tokio::test]
    async fn test_next_plasma_non_plasma_tx_data_forwards() {
        let trace_store: TraceStorage = Default::default();
        let layer = CollectingLayer::new(trace_store.clone());
        tracing_subscriber::Registry::default().with(layer).init();

        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher { advances: vec![Ok(())], ..Default::default() };
        let first = Bytes::copy_from_slice(&[2u8]);
        let source = vec![first.clone()].into_iter();
        let id = BlockID { number: 1, ..Default::default() };

        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let data = plasma_source.next().await.unwrap();
        assert_eq!(data, first);

        let logs = trace_store.get_by_level(Level::INFO);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("non-plasma tx data, forwarding downstream"));
    }

    // TODO: more tests
}
