//! Contains the `PipelineBuilder` object that is used to build a `DerivationPipeline`.

use super::{
    AttributesBuilder, ChainProvider, DataAvailabilityProvider, DerivationPipeline,
    L2ChainProvider, ResetProvider,
};
use crate::stages::{
    AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue, L1Retrieval, L1Traversal,
};
use alloc::{collections::VecDeque, sync::Arc};
use core::fmt::Debug;
use kona_primitives::{L2BlockInfo, RollupConfig};

type L1TraversalStage<P> = L1Traversal<P>;
type L1RetrievalStage<DAP, P> = L1Retrieval<DAP, L1TraversalStage<P>>;
type FrameQueueStage<DAP, P> = FrameQueue<L1RetrievalStage<DAP, P>>;
type ChannelBankStage<DAP, P> = ChannelBank<FrameQueueStage<DAP, P>>;
type ChannelReaderStage<DAP, P> = ChannelReader<ChannelBankStage<DAP, P>>;
type BatchQueueStage<DAP, P, T> = BatchQueue<ChannelReaderStage<DAP, P>, T>;
type AttributesQueueStage<DAP, P, T, B> = AttributesQueue<BatchQueueStage<DAP, P, T>, B>;

/// The `PipelineBuilder` constructs a [DerivationPipeline] using a builder pattern.
///
/// ## Usage
///
/// ```rust
/// #![cfg(feature = "online")]
///
/// use alloc::sync::Arc;
/// use alloy_provider::ReqwestProvider;
/// use alloy_rpc_client::RpcClient;
/// use alloy_transport_http::Http;
/// use kona_derive::{
///     online::{
///         AlloyChainProvider, AlloyL2ChainProvider, OnlineBeaconClient, OnlineBlobProvider,
///         SimpleSlotDerivation,
///     },
///     pipeline::*,
/// };
/// use reqwest::Client;
///
/// #[allow(clippy::needless_doctest_main)]
/// fn main() {
///     // Creates a new chain provider using the `L1_RPC_URL` environment variable.
///     let l1_rpc_url = std::env::var("L1_RPC_URL").expect("L1_RPC_URL must be set");
///     let l1_rpc_url = l1_rpc_url.parse().unwrap();
///     let http = Http::<Client>::new(l1_rpc_url);
///     let chain_provider =
///         AlloyChainProvider::new(ReqwestProvider::new(RpcClient::new(http, true)));
///
///     // Creates a new l2 chain provider using the `L2_RPC_URL` environment variable.
///     let l2_rpc_url = std::env::var("L2_RPC_URL").expect("L2_RPC_URL must be set");
///     let l2_rpc_url = l2_rpc_url.parse().unwrap();
///     let http = Http::<Client>::new(l2_rpc_url);
///     let l2_chain_provider =
///         AlloyL2ChainProvider::new(ReqwestProvider::new(RpcClient::new(http, true)));
///
///     // TODO(refcell): replace this will a rollup config
///     // fetched from the superchain-registry via network id.
///     let rollup_config = Arc::new(RollupConfig::default());
///
///     // Create the beacon client used to fetch blob data.
///     let beacon_url = std::env::var("BEACON_URL").expect("BEACON_URL must be set");
///     let beacon_url = beacon_url.parse().unwrap();
///     let http = Http::<Client>::new(beacon_url);
///     let beacon_client =
///         OnlineBeaconClient::new(ReqwestProvider::new(RpcClient::new(http, true)));
///
///     // Build the online blob provider.
///     let blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
///         OnlineBlobProvider::new(true, beacon_client, None, None);
///
///     // Build the ethereum data source
///     let dap_source =
///         EthereumDataSource::new(chain_provider.clone(), blob_provider, &rollup_config);
///
///     let builder = PipelineBuilder::new();
///     let pipeline = builder
///         .rollup_config(rollup_config)
///         .dap_source(dap_source)
///         .l2_chain_provider(l2_chain_provider)
///         .chain_provider(chain_provider)
///         .builder(OnlineAttributesBuilder::new())
///         .reset(ResetProvider::new())
///         .start_cursor(L2BlockInfo::default())
///         .build();
///
///     assert_eq!(pipeline.needs_reset, false);
/// }
/// ```
#[derive(Debug)]
pub struct PipelineBuilder<R, B, P, T, D>
where
    R: ResetProvider + Send + Debug,
    B: AttributesBuilder + Send + Debug,
    P: ChainProvider + Send + Sync + Debug,
    T: L2ChainProvider + Send + Sync + Debug,
    D: DataAvailabilityProvider + Send + Sync + Debug,
{
    l2_chain_provider: Option<T>,
    dap_source: Option<D>,
    chain_provider: Option<P>,
    builder: Option<B>,
    rollup_config: Option<Arc<RollupConfig>>,
    reset: Option<R>,
    start_cursor: Option<L2BlockInfo>,
}

impl<R, B, P, T, D> Default for PipelineBuilder<R, B, P, T, D>
where
    R: ResetProvider + Send + Debug,
    B: AttributesBuilder + Send + Debug,
    P: ChainProvider + Send + Sync + Debug,
    T: L2ChainProvider + Send + Sync + Debug,
    D: DataAvailabilityProvider + Send + Sync + Debug,
{
    fn default() -> Self {
        Self {
            l2_chain_provider: None,
            dap_source: None,
            chain_provider: None,
            builder: None,
            rollup_config: None,
            reset: None,
            start_cursor: None,
        }
    }
}

impl<R, B, P, T, D> PipelineBuilder<R, B, P, T, D>
where
    R: ResetProvider + Send + Debug,
    B: AttributesBuilder + Send + Debug,
    P: ChainProvider + Send + Sync + Debug,
    T: L2ChainProvider + Send + Sync + Debug,
    D: DataAvailabilityProvider + Send + Sync + Debug,
{
    /// Creates a new pipeline builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the rollup config for the pipeline.
    pub fn rollup_config(mut self, rollup_config: Arc<RollupConfig>) -> Self {
        self.rollup_config = Some(rollup_config);
        self
    }

    /// Sets the data availability provider for the pipeline.
    pub fn dap_source(mut self, dap_source: D) -> Self {
        self.dap_source = Some(dap_source);
        self
    }

    /// Sets the builder for the pipeline.
    pub fn builder(mut self, builder: B) -> Self {
        self.builder = Some(builder);
        self
    }

    /// Sets the l2 chain provider for the pipeline.
    pub fn l2_chain_provider(mut self, l2_chain_provider: T) -> Self {
        self.l2_chain_provider = Some(l2_chain_provider);
        self
    }

    /// Sets the chain provider for the pipeline.
    pub fn chain_provider(mut self, chain_provider: P) -> Self {
        self.chain_provider = Some(chain_provider);
        self
    }

    /// Sets the reset provider for the pipeline.
    pub fn reset(mut self, reset: R) -> Self {
        self.reset = Some(reset);
        self
    }

    /// Sets the start cursor for the pipeline.
    pub fn start_cursor(mut self, cursor: L2BlockInfo) -> Self {
        self.start_cursor = Some(cursor);
        self
    }

    /// Builds the pipeline.
    pub fn build(self) -> DerivationPipeline<AttributesQueueStage<D, P, T, B>, R> {
        self.into()
    }
}

impl<R, B, P, T, D> From<PipelineBuilder<R, B, P, T, D>>
    for DerivationPipeline<AttributesQueueStage<D, P, T, B>, R>
where
    R: ResetProvider + Send + Debug,
    B: AttributesBuilder + Send + Debug,
    P: ChainProvider + Send + Sync + Debug,
    T: L2ChainProvider + Send + Sync + Debug,
    D: DataAvailabilityProvider + Send + Sync + Debug,
{
    fn from(builder: PipelineBuilder<R, B, P, T, D>) -> Self {
        // Extract the builder fields.
        let rollup_config = builder.rollup_config.expect("rollup_config must be set");
        let chain_provider = builder.chain_provider.expect("chain_provider must be set");
        let l2_chain_provider = builder.l2_chain_provider.expect("chain_provider must be set");
        let dap_source = builder.dap_source.expect("dap_source must be set");
        let reset = builder.reset.expect("reset must be set");
        let attributes_builder = builder.builder.expect("builder must be set");

        // Compose the stage stack.
        let l1_traversal = L1Traversal::new(chain_provider, Arc::clone(&rollup_config));
        let l1_retrieval = L1Retrieval::new(l1_traversal, dap_source);
        let frame_queue = FrameQueue::new(l1_retrieval);
        let channel_bank = ChannelBank::new(Arc::clone(&rollup_config), frame_queue);
        let channel_reader = ChannelReader::new(channel_bank, Arc::clone(&rollup_config));
        let batch_queue = BatchQueue::new(rollup_config.clone(), channel_reader, l2_chain_provider);
        let attributes = AttributesQueue::new(*rollup_config, batch_queue, attributes_builder);

        // Create the pipeline.
        DerivationPipeline {
            attributes,
            reset,
            prepared: VecDeque::new(),
            needs_reset: false,
            cursor: builder.start_cursor.unwrap_or_default(),
        }
    }
}
