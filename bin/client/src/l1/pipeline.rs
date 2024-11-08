//! Contains an oracle-backed pipeline for

use alloc::{boxed::Box, sync::Arc};
use alloy_consensus::Sealed;
use async_trait::async_trait;
use core::fmt::Debug;
use kona_derive::{
    attributes::StatefulAttributesBuilder,
    errors::PipelineErrorKind,
    pipeline::{DerivationPipeline, PipelineBuilder},
    sources::EthereumDataSource,
    stages::{
        AttributesQueue, BatchProvider, BatchStream, ChannelProvider, ChannelReader, FrameQueue,
        L1Retrieval, L1Traversal,
    },
    traits::{BlobProvider, ChainProvider, OriginProvider, Pipeline, SignalReceiver},
    types::{PipelineResult, Signal, StepResult},
};
use kona_driver::{DriverPipeline, SyncCursor};
use kona_mpt::TrieProvider;
use kona_preimage::{CommsClient, PreimageKey, PreimageKeyType};
use op_alloy_genesis::{RollupConfig, SystemConfig};
use op_alloy_protocol::{BatchValidationProvider, BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::OpAttributesWithParent;

use crate::{
    errors::OracleProviderError, l1::OracleL1ChainProvider, l2::OracleL2ChainProvider, BootInfo,
    FlushableCache, HintType,
};

/// An oracle-backed derivation pipeline.
pub type OracleDerivationPipeline<O, B> = DerivationPipeline<
    OracleAttributesQueue<OracleDataProvider<O, B>, O>,
    OracleL2ChainProvider<O>,
>;

/// An oracle-backed Ethereum data source.
pub type OracleDataProvider<O, B> = EthereumDataSource<OracleL1ChainProvider<O>, B>;

/// An oracle-backed payload attributes builder for the `AttributesQueue` stage of the derivation
/// pipeline.
pub type OracleAttributesBuilder<O> =
    StatefulAttributesBuilder<OracleL1ChainProvider<O>, OracleL2ChainProvider<O>>;

/// An oracle-backed attributes queue for the derivation pipeline.
pub type OracleAttributesQueue<DAP, O> = AttributesQueue<
    BatchProvider<
        BatchStream<
            ChannelReader<
                ChannelProvider<
                    FrameQueue<L1Retrieval<DAP, L1Traversal<OracleL1ChainProvider<O>>>>,
                >,
            >,
            OracleL2ChainProvider<O>,
        >,
        OracleL2ChainProvider<O>,
    >,
    OracleAttributesBuilder<O>,
>;

/// An error encountered when starting the pipeline
#[derive(Debug, derive_more::Display)]
pub enum PipelineStartError {
    /// An oracle provider error.
    #[display("Oracle provider error: {_0}")]
    OracleProvider(OracleProviderError),
}

impl core::error::Error for PipelineStartError {}

impl From<OracleProviderError> for PipelineStartError {
    fn from(err: OracleProviderError) -> Self {
        Self::OracleProvider(err)
    }
}

/// An oracle-backed derivation pipeline.
#[derive(Debug)]
pub struct OraclePipeline<O, B>
where
    O: CommsClient + FlushableCache + Send + Sync + Debug,
    B: BlobProvider + Send + Sync + Debug + Clone,
{
    /// The internal derivation pipeline.
    pub pipeline: OracleDerivationPipeline<O, B>,
    /// The caching oracle.
    pub caching_oracle: Arc<O>,
}

impl<O, B> OraclePipeline<O, B>
where
    O: CommsClient + FlushableCache + FlushableCache + Send + Sync + Debug,
    B: BlobProvider + Send + Sync + Debug + Clone,
{
    /// Constructs a new oracle-backed derivation pipeline.
    pub async fn new(
        boot_info: &BootInfo,
        caching_oracle: Arc<O>,
        blob_provider: B,
        mut chain_provider: OracleL1ChainProvider<O>,
        mut l2_chain_provider: OracleL2ChainProvider<O>,
    ) -> Result<(Self, SyncCursor), PipelineStartError> {
        let cfg = Arc::new(boot_info.rollup_config.clone());

        // Fetch the startup information.
        let (l1_origin, l2_safe_head, sc) = Self::sync_start(
            caching_oracle.clone(),
            boot_info,
            &mut chain_provider,
            &mut l2_chain_provider,
        )
        .await?;

        // Walk back the starting L1 block by `channel_timeout` to ensure that the full channel is
        // captured.
        let channel_timeout =
            boot_info.rollup_config.channel_timeout(l2_safe_head.block_info.timestamp);
        let mut l1_origin_number = l1_origin.number.saturating_sub(channel_timeout);
        if l1_origin_number < boot_info.rollup_config.genesis.l1.number {
            l1_origin_number = boot_info.rollup_config.genesis.l1.number;
        }
        let l1_origin = chain_provider.block_info_by_number(l1_origin_number).await?;

        let attributes = StatefulAttributesBuilder::new(
            cfg.clone(),
            l2_chain_provider.clone(),
            chain_provider.clone(),
        );
        let dap = EthereumDataSource::new_from_parts(chain_provider.clone(), blob_provider, &cfg);

        let pipeline = PipelineBuilder::new()
            .rollup_config(cfg)
            .dap_source(dap)
            .l2_chain_provider(l2_chain_provider)
            .chain_provider(chain_provider)
            .builder(attributes)
            .origin(l1_origin)
            .build();
        Ok((Self { pipeline, caching_oracle }, sc))
    }

    async fn sync_start(
        caching_oracle: Arc<O>,
        boot_info: &BootInfo,
        chain_provider: &mut OracleL1ChainProvider<O>,
        l2_chain_provider: &mut OracleL2ChainProvider<O>,
    ) -> Result<(BlockInfo, L2BlockInfo, SyncCursor), PipelineStartError> {
        // Find the initial safe head, based off of the starting L2 block number in the boot info.
        caching_oracle
            .write(
                &HintType::StartingL2Output
                    .encode_with(&[boot_info.agreed_l2_output_root.as_ref()]),
            )
            .await
            .map_err(OracleProviderError::Preimage)?;
        let mut output_preimage = [0u8; 128];
        caching_oracle
            .get_exact(
                PreimageKey::new(*boot_info.agreed_l2_output_root, PreimageKeyType::Keccak256),
                &mut output_preimage,
            )
            .await
            .map_err(OracleProviderError::Preimage)?;

        let safe_hash =
            output_preimage[96..128].try_into().map_err(OracleProviderError::SliceConversion)?;
        let safe_header = l2_chain_provider.header_by_hash(safe_hash)?;
        let safe_head_info = l2_chain_provider.l2_block_info_by_number(safe_header.number).await?;
        let l1_origin =
            chain_provider.block_info_by_number(safe_head_info.l1_origin.number).await?;

        Ok((
            l1_origin,
            safe_head_info,
            SyncCursor::new(
                safe_head_info,
                Sealed::new_unchecked(safe_header, safe_hash),
                boot_info.agreed_l2_output_root,
            ),
        ))
    }
}

impl<O, B> DriverPipeline<OracleDerivationPipeline<O, B>> for OraclePipeline<O, B>
where
    O: CommsClient + FlushableCache + Send + Sync + Debug,
    B: BlobProvider + Send + Sync + Debug + Clone,
{
    /// Flushes the cache on re-org.
    fn flush(&self) {
        self.caching_oracle.flush();
    }
}

#[async_trait]
impl<O, B> SignalReceiver for OraclePipeline<O, B>
where
    O: CommsClient + FlushableCache + Send + Sync + Debug,
    B: BlobProvider + Send + Sync + Debug + Clone,
{
    /// Receives a signal from the driver.
    async fn signal(&mut self, signal: Signal) -> PipelineResult<()> {
        self.pipeline.signal(signal).await
    }
}

impl<O, B> OriginProvider for OraclePipeline<O, B>
where
    O: CommsClient + FlushableCache + Send + Sync + Debug,
    B: BlobProvider + Send + Sync + Debug + Clone,
{
    /// Returns the optional L1 [BlockInfo] origin.
    fn origin(&self) -> Option<BlockInfo> {
        self.pipeline.origin()
    }
}

impl<O, B> Iterator for OraclePipeline<O, B>
where
    O: CommsClient + FlushableCache + Send + Sync + Debug,
    B: BlobProvider + Send + Sync + Debug + Clone,
{
    type Item = OpAttributesWithParent;

    fn next(&mut self) -> Option<Self::Item> {
        self.pipeline.next()
    }
}

#[async_trait]
impl<O, B> Pipeline for OraclePipeline<O, B>
where
    O: CommsClient + FlushableCache + Send + Sync + Debug,
    B: BlobProvider + Send + Sync + Debug + Clone,
{
    /// Peeks at the next [OpAttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&OpAttributesWithParent> {
        self.pipeline.peek()
    }

    /// Attempts to progress the pipeline.
    async fn step(&mut self, cursor: L2BlockInfo) -> StepResult {
        self.pipeline.step(cursor).await
    }

    /// Returns the rollup config.
    fn rollup_config(&self) -> &RollupConfig {
        self.pipeline.rollup_config()
    }

    /// Returns the [SystemConfig] by L2 number.
    async fn system_config_by_number(
        &mut self,
        number: u64,
    ) -> Result<SystemConfig, PipelineErrorKind> {
        self.pipeline.system_config_by_number(number).await
    }
}
