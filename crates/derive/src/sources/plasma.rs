//! Plasma Data Source

use crate::{
    sources::BaseDataSource,
    traits::{AsyncIterator, BlobProvider},
    types::{ResetError, StageError, StageResult},
};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_plasma::{
    traits::PlasmaInputFetcher,
    types::{
        decode_keccak256, Keccak256Commitment, PlasmaError, MAX_INPUT_SIZE, TX_DATA_VERSION_1,
    },
};
use kona_primitives::block::BlockID;
use kona_providers::ChainProvider;

/// A plasma data iterator.
#[derive(Debug, Clone)]
pub struct PlasmaSource<CP, B, PIF>
where
    CP: ChainProvider + Send + Clone,
    B: BlobProvider + Send + Clone,
    PIF: PlasmaInputFetcher<CP> + Send,
{
    /// The plasma input fetcher.
    input_fetcher: PIF,
    /// The chain provider to use for the plasma source.
    chain_provider: CP,
    /// A source data iterator.
    source: BaseDataSource<CP, B>,
    /// Keeps track of a pending commitment so we can keep trying to fetch the input.
    commitment: Option<Keccak256Commitment>,
    /// The block Id.
    id: BlockID,
}

impl<CP, B, PIF> PlasmaSource<CP, B, PIF>
where
    CP: ChainProvider + Send + Clone,
    B: BlobProvider + Send + Clone,
    PIF: PlasmaInputFetcher<CP> + Send,
{
    /// Instantiates a new plasma data source.
    pub fn new(
        chain_provider: CP,
        input_fetcher: PIF,
        source: BaseDataSource<CP, B>,
        id: BlockID,
    ) -> Self {
        Self { chain_provider, input_fetcher, source, id, commitment: None }
    }
}

#[async_trait]
impl<CP, B, PIF> AsyncIterator for PlasmaSource<CP, B, PIF>
where
    CP: ChainProvider + Send + Clone,
    B: BlobProvider + Send + Clone,
    PIF: PlasmaInputFetcher<CP> + Send,
{
    type Item = Bytes;

    async fn next(&mut self) -> Option<StageResult<Self::Item>> {
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
                return Some(StageResult::Err(StageError::Reset(ResetError::NewExpiredChallenge)));
            }
            Some(Err(e)) => {
                tracing::error!("failed to advance plasma L1 origin: {:?}", e);
                return Some(StageResult::Err(StageError::Temporary(anyhow::anyhow!(
                    "failed to advance plasma L1 origin: {:?}",
                    e
                ))));
            }
            None => {
                tracing::warn!("l1 origin advance returned None");
            }
        }

        // Set the commitment if it isn't available.
        if self.commitment.is_none() {
            // The l1 source returns the input commitment for the batch.
            let data = match self.source.next().await.ok_or(PlasmaError::NotEnoughData) {
                Ok(Ok(d)) => d,
                Ok(Err(e)) => {
                    tracing::warn!("failed to pull next data from the plasma source iterator");
                    return Some(Err(e));
                }
                Err(e) => {
                    tracing::warn!("failed to pull next data from the plasma source iterator");
                    return Some(Err(StageError::Plasma(e)));
                }
            };

            // If the data is empty,
            if data.is_empty() {
                tracing::warn!("empty data from plasma source");
                return Some(Err(StageError::Plasma(PlasmaError::NotEnoughData)));
            }

            // If the tx data type is not plasma, we forward it downstream to let the next
            // steps validate and potentially parse it as L1 DA inputs.
            if data[0] != TX_DATA_VERSION_1 {
                tracing::info!("non-plasma tx data, forwarding downstream");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        sources::calldata::CalldataSource,
        stages::test_utils::{CollectingLayer, TraceStorage},
        traits::test_utils::TestBlobProvider,
    };
    use alloc::{vec, vec::Vec};
    use alloy_consensus::{SignableTransaction, TxEip1559, TxEnvelope};
    use alloy_primitives::{Address, Signature, TxKind, U256};
    use kona_plasma::test_utils::TestPlasmaInputFetcher;
    use kona_primitives::BlockInfo;
    use kona_providers::test_utils::TestChainProvider;
    use tracing::Level;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    #[tokio::test]
    async fn test_next_plasma_advance_origin_reorg_error() {
        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher {
            advances: vec![Err(PlasmaError::ReorgRequired)],
            ..Default::default()
        };
        let source: BaseDataSource<TestChainProvider, TestBlobProvider> =
            BaseDataSource::Calldata(CalldataSource::new(
                chain_provider.clone(),
                Address::default(),
                BlockInfo::default(),
                Address::default(),
            ));
        let id = BlockID { number: 1, ..Default::default() };

        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap().unwrap_err();
        assert_eq!(err, StageError::Reset(ResetError::NewExpiredChallenge));
    }

    #[tokio::test]
    async fn test_next_plasma_advance_origin_other_error() {
        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher {
            advances: vec![Err(PlasmaError::NotEnoughData)],
            ..Default::default()
        };
        let source: BaseDataSource<TestChainProvider, TestBlobProvider> =
            BaseDataSource::Calldata(CalldataSource::new(
                chain_provider.clone(),
                Address::default(),
                BlockInfo::default(),
                Address::default(),
            ));
        let id = BlockID { number: 1, ..Default::default() };

        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap().unwrap_err();
        matches!(err, StageError::Temporary(_));
    }

    #[tokio::test]
    async fn test_next_plasma_internal_block_fetch_fail() {
        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher { advances: vec![Ok(())], ..Default::default() };
        let id = BlockID { number: 1, ..Default::default() };
        let source: BaseDataSource<TestChainProvider, TestBlobProvider> =
            BaseDataSource::Calldata(CalldataSource::new(
                chain_provider.clone(),
                Address::default(),
                BlockInfo::default(),
                Address::default(),
            ));

        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap().unwrap_err();
        assert_eq!(err, StageError::BlockFetch(Default::default()));
    }

    #[tokio::test]
    async fn test_next_plasma_calldata_eof() {
        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher { advances: vec![Ok(())], ..Default::default() };
        let id = BlockID { number: 1, ..Default::default() };
        let source_chain_provider = TestChainProvider {
            blocks: vec![(1, BlockInfo::default(), Vec::new())],
            ..Default::default()
        };
        let source: BaseDataSource<TestChainProvider, TestBlobProvider> =
            BaseDataSource::Calldata(CalldataSource::new(
                source_chain_provider,
                Address::default(),
                BlockInfo::default(),
                Address::default(),
            ));
        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap().unwrap_err();
        assert_eq!(err, StageError::Eof);
    }

    #[tokio::test]
    async fn test_next_plasma_not_enough_source_data() {
        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher { advances: vec![Ok(())], ..Default::default() };
        let id = BlockID { number: 1, ..Default::default() };
        let signature = Signature::test_signature();
        let batcher_address = Address::left_padding_from(&[6]);
        let tx = TxEnvelope::Eip1559(
            TxEip1559 {
                chain_id: 1u64,
                nonce: 2,
                max_fee_per_gas: 3,
                max_priority_fee_per_gas: 4,
                gas_limit: 5,
                to: TxKind::Call(batcher_address),
                value: U256::from(7_u64),
                input: Bytes::from(vec![]),
                access_list: Default::default(),
            }
            .into_signed(signature),
        );
        let signer = alloy_primitives::address!("616268d0e4d1a33d8f95aba56e880b6e29551174");
        let txs = vec![tx];
        let source_chain_provider = TestChainProvider {
            blocks: vec![(1, BlockInfo::default(), txs)],
            ..Default::default()
        };
        let source: BaseDataSource<TestChainProvider, TestBlobProvider> =
            BaseDataSource::Calldata(CalldataSource::new(
                source_chain_provider,
                batcher_address,
                BlockInfo::default(),
                signer,
            ));
        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap().unwrap_err();
        // We cant assert NotEnoughData here since we can't force the calldata source to pop
        // nothing.
        assert_eq!(err, StageError::Eof);
    }

    #[tokio::test]
    async fn test_next_plasma_non_plasma_tx_data_forwards() {
        let trace_store: TraceStorage = Default::default();
        let layer = CollectingLayer::new(trace_store.clone());
        tracing_subscriber::Registry::default().with(layer).init();

        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher { advances: vec![Ok(())], ..Default::default() };
        let id = BlockID { number: 1, ..Default::default() };
        let signature = Signature::test_signature();
        let batcher_address = Address::left_padding_from(&[6]);
        let tx = TxEnvelope::Eip1559(
            TxEip1559 {
                chain_id: 1u64,
                nonce: 2,
                max_fee_per_gas: 3,
                max_priority_fee_per_gas: 4,
                gas_limit: 5,
                to: TxKind::Call(batcher_address),
                value: U256::from(7_u64),
                input: Bytes::from(vec![8]),
                access_list: Default::default(),
            }
            .into_signed(signature),
        );
        let signer = alloy_primitives::address!("616268d0e4d1a33d8f95aba56e880b6e29551174");
        let txs = vec![tx];
        let source_chain_provider = TestChainProvider {
            blocks: vec![(1, BlockInfo::default(), txs)],
            ..Default::default()
        };
        let source: BaseDataSource<TestChainProvider, TestBlobProvider> =
            BaseDataSource::Calldata(CalldataSource::new(
                source_chain_provider,
                batcher_address,
                BlockInfo::default(),
                signer,
            ));
        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let data = plasma_source.next().await.unwrap().unwrap();
        assert_eq!(data, vec![8u8]);

        let logs = trace_store.get_by_level(Level::INFO);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("non-plasma tx data, forwarding downstream"));
    }

    #[tokio::test]
    async fn test_next_plasma_valid_commitment_failed_to_pull_next_data() {
        let trace_store: TraceStorage = Default::default();
        let layer = CollectingLayer::new(trace_store.clone());
        tracing_subscriber::Registry::default().with(layer).init();

        let chain_provider = TestChainProvider::default();
        let input_fetcher = TestPlasmaInputFetcher { advances: vec![Ok(())], ..Default::default() };
        let id = BlockID { number: 1, ..Default::default() };
        let signature = Signature::test_signature();
        let batcher_address = Address::left_padding_from(&[6]);
        let input = Bytes::from(
            &b"01001d2b0bda21d56b8bd12d4f94ebacffdfb35f5e226f84b461103bb8beab6353be"[..],
        );
        let tx = TxEnvelope::Eip1559(
            TxEip1559 {
                chain_id: 1u64,
                nonce: 2,
                max_fee_per_gas: 3,
                max_priority_fee_per_gas: 4,
                gas_limit: 5,
                to: TxKind::Call(batcher_address),
                value: U256::from(7_u64),
                input,
                access_list: Default::default(),
            }
            .into_signed(signature),
        );
        let signer = alloy_primitives::address!("616268d0e4d1a33d8f95aba56e880b6e29551174");
        let txs = vec![tx];
        let source_chain_provider = TestChainProvider {
            blocks: vec![(1, BlockInfo::default(), txs)],
            ..Default::default()
        };
        let source: BaseDataSource<TestChainProvider, TestBlobProvider> =
            BaseDataSource::Calldata(CalldataSource::new(
                source_chain_provider,
                batcher_address,
                BlockInfo::default(),
                signer,
            ));
        let mut plasma_source = PlasmaSource::new(chain_provider, input_fetcher, source, id);

        let err = plasma_source.next().await.unwrap().unwrap_err();
        assert_eq!(err, StageError::Eof);

        let logs = trace_store.get_by_level(Level::WARN);
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("failed to pull next data from the plasma source iterator"));
    }

    // TODO: more tests
}
