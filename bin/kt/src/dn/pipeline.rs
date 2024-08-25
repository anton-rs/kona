//! The pipeline module contains the pipeline logic for the test runner.

use super::{
    blobs::BlobFixtureProvider,
    providers::{FixtureL1Provider, FixtureL2Provider},
    LocalDerivationFixture,
};
use anyhow::{anyhow, Result};
use kona_derive::{
    pipeline::{DerivationPipeline, PipelineBuilder},
    sources::EthereumDataSource,
    stages::{
        AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue, L1Retrieval,
        L1Traversal, StatefulAttributesBuilder,
    },
    traits::ChainProvider,
};
use std::sync::Arc;
use tracing::info;

/// The test runner derivation pipeline.
pub(crate) type RunnerPipeline =
    DerivationPipeline<RunnerAttributesQueue<RunnerDataProvider>, FixtureL2Provider>;

/// A test runner Ethereum data provider.
pub(crate) type RunnerDataProvider = EthereumDataSource<FixtureL1Provider, BlobFixtureProvider>;

/// A test runner payload attributes builder for the `AttributesQueue` stage of the derivation
/// pipeline.
pub(crate) type RunnerAttributesBuilder =
    StatefulAttributesBuilder<FixtureL1Provider, FixtureL2Provider>;

/// A test runner attributes queue for the derivation pipeline.
pub(crate) type RunnerAttributesQueue<DAP> = AttributesQueue<
    BatchQueue<
        ChannelReader<ChannelBank<FrameQueue<L1Retrieval<DAP, L1Traversal<FixtureL1Provider>>>>>,
        FixtureL2Provider,
    >,
    RunnerAttributesBuilder,
>;

/// Creates a new [DerivationPipeline] given the [LocalDerivationFixture].
pub(crate) async fn new_runner_pipeline(fixture: LocalDerivationFixture) -> Result<RunnerPipeline> {
    let mut l1_provider = FixtureL1Provider::from(fixture.clone());
    let l2_provider = FixtureL2Provider::from(fixture.clone());
    let blob_provider = BlobFixtureProvider::from(fixture.clone());
    let cfg = Arc::new(fixture.rollup_config.clone());
    let dap = EthereumDataSource::new(l1_provider.clone(), blob_provider, &cfg);
    let attributes =
        StatefulAttributesBuilder::new(cfg.clone(), l2_provider.clone(), l1_provider.clone());

    // Range we want to derive.
    let Some(start) = fixture.l2_payloads.keys().min() else {
        return Err(anyhow!("No blocks found"));
    };
    let Some(end) = fixture.l2_payloads.keys().max() else {
        return Err(anyhow!("No blocks found"));
    };
    info!(target: "exec", "Deriving blocks {} to {}", start, end);

    // Get the cursor from the l2 block infos.
    let Some(cursor) = fixture.l2_block_infos.get(start) else {
        return Err(anyhow!("Cursor not found"));
    };

    let origin = l1_provider.block_info_by_number(cursor.l1_origin.number).await?;

    Ok(PipelineBuilder::new()
        .rollup_config(cfg)
        .dap_source(dap)
        .l2_chain_provider(l2_provider)
        .chain_provider(l1_provider)
        .builder(attributes)
        .origin(origin)
        .build())
}
