//! Plasma Data Source

use crate::{
    traits::AsyncIterator,
    types::{ResetError, StageError, StageResult},
};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_plasma::{
    traits::{ChainProvider, PlasmaInputFetcher},
    types::{
        decode_keccak256, Keccak256Commitment, PlasmaError, MAX_INPUT_SIZE, TX_DATA_VERSION_1,
    },
};
use kona_primitives::block::BlockID;

/// A plasma data iterator.
#[derive(Debug, Clone)]
pub struct PlasmaSource<CP, PIF, I>
where
    CP: ChainProvider + Send,
    PIF: PlasmaInputFetcher<CP> + Send,
    I: Iterator<Item = Bytes>,
{
    /// The plasma input fetcher.
    input_fetcher: PIF,
    /// The chain provider to use for the plasma source.
    chain_provider: CP,
    /// A source data iterator.
    source: I,
    /// Keeps track of a pending commitment so we can keep trying to fetch the input.
    commitment: Option<Keccak256Commitment>,
    /// The block Id.
    id: BlockID,
}

impl<CP, PIF, I> PlasmaSource<CP, PIF, I>
where
    CP: ChainProvider + Send,
    PIF: PlasmaInputFetcher<CP> + Send,
    I: Iterator<Item = Bytes>,
{
    /// Instantiates a new plasma data source.
    pub fn new(chain_provider: CP, input_fetcher: PIF, source: I, id: BlockID) -> Self {
        Self { chain_provider, input_fetcher, source, id, commitment: None }
    }
}

#[async_trait]
impl<CP, PIF, I> AsyncIterator for PlasmaSource<CP, PIF, I>
where
    CP: ChainProvider + Send,
    PIF: PlasmaInputFetcher<CP> + Send,
    I: Iterator<Item = Bytes> + Send,
{
    type Item = Bytes;

    async fn next(&mut self) -> Option<StageResult<Self::Item>> {
        // Process origin syncs the challenge contract events and updates the local challenge states
        // before we can proceed to fetch the input data. This function can be called multiple times
        // for the same origin and noop if the origin was already processed. It is also called if
        // there is not commitment in the current origin.
        match self.input_fetcher.advance_l1_origin(&self.chain_provider, self.id).await {
            Some(Ok(_)) => (),
            Some(Err(PlasmaError::ReorgRequired)) => {
                tracing::error!("new expired challenge");
                return Some(StageResult::Err(StageError::Custom(anyhow::anyhow!(
                    "new expired challenge"
                ))));
            }
            Some(Err(e)) => {
                tracing::error!("failed to advance plasma L1 origin: {:?}", e);
                return Some(StageResult::Err(StageError::Plasma(e)));
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
                    return Some(Err(StageError::Plasma(e)));
                }
            };

            // If the data is empty,
            if data.is_empty() {
                return Some(Err(StageError::Plasma(PlasmaError::NotEnoughData)));
            }

            // If the tx data type is not plasma, we forward it downstream to let the next
            // steps validate and potentially parse it as L1 DA inputs.
            if data[0] != TX_DATA_VERSION_1 {
                return Some(Ok(data));
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
                return Some(Err(StageError::Reset(ResetError::ReorgRequired)));
            }
            Some(Err(PlasmaError::ChallengeExpired)) => {
                // This commitment was challenged and the challenge expired.
                tracing::warn!("challenge expired, skipping batch");
                self.commitment = None;
                // Skip the input.
                return self.next().await
            }
            Some(Err(PlasmaError::MissingPastWindow)) => {
                tracing::warn!("missing past window, skipping batch");
                return Some(Err(StageError::Critical(anyhow::anyhow!(
                    "data for commitment {:?} not available",
                    commitment
                ))));
            }
            Some(Err(PlasmaError::ChallengePending)) => {
                // Continue stepping without slowing down.
                tracing::debug!("plasma challenge pending, proceeding");
                return Some(Err(StageError::NotEnoughData));
            }
            Some(Err(e)) => {
                // Return temporary error so we can keep retrying.
                return Some(Err(StageError::Temporary(anyhow::anyhow!(
                    "failed to fetch input data with comm {:?} from da service: {:?}",
                    commitment,
                    e
                ))));
            }
            None => {
                // Return temporary error so we can keep retrying.
                return Some(Err(StageError::Temporary(anyhow::anyhow!(
                    "failed to fetch input data with comm {:?} from da service",
                    commitment
                ))));
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

        return Some(Ok(data));
    }
}
