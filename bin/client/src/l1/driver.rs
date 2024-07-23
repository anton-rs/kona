//! Contains the [DerivationDriver] struct, which handles the [L2PayloadAttributes] derivation
//! process.
//!
//! [L2PayloadAttributes]: kona_derive::types::L2PayloadAttributes

use super::{OracleBlobProvider, OracleL1ChainProvider};
use crate::{l2::OracleL2ChainProvider, BootInfo, HintType};
use alloc::sync::Arc;
use alloy_consensus::{Header, Sealed};
use anyhow::{anyhow, Result};
use core::fmt::Debug;
use kona_derive::{
    pipeline::{DerivationPipeline, Pipeline, PipelineBuilder, StepResult},
    sources::EthereumDataSource,
    stages::{
        AttributesQueue, BatchQueue, ChannelBank, ChannelReader, FrameQueue, L1Retrieval,
        L1Traversal, StatefulAttributesBuilder,
    },
    traits::{ChainProvider, L2ChainProvider},
};
use kona_mpt::TrieDBFetcher;
use kona_preimage::{CommsClient, PreimageKey, PreimageKeyType};
use kona_primitives::{BlockInfo, L2AttributesWithParent, L2BlockInfo};
use tracing::{info, warn};

/// An oracle-backed derivation pipeline.
pub type OraclePipeline<O> =
    DerivationPipeline<OracleAttributesQueue<OracleDataProvider<O>, O>, OracleL2ChainProvider<O>>;

/// An oracle-backed Ethereum data source.
pub type OracleDataProvider<O> =
    EthereumDataSource<OracleL1ChainProvider<O>, OracleBlobProvider<O>>;

/// An oracle-backed payload attributes builder for the `AttributesQueue` stage of the derivation
/// pipeline.
pub type OracleAttributesBuilder<O> =
    StatefulAttributesBuilder<OracleL1ChainProvider<O>, OracleL2ChainProvider<O>>;

/// An oracle-backed attributes queue for the derivation pipeline.
pub type OracleAttributesQueue<DAP, O> = AttributesQueue<
    BatchQueue<
        ChannelReader<
            ChannelBank<FrameQueue<L1Retrieval<DAP, L1Traversal<OracleL1ChainProvider<O>>>>>,
        >,
        OracleL2ChainProvider<O>,
    >,
    OracleAttributesBuilder<O>,
>;

/// The [DerivationDriver] struct is responsible for handling the [L2PayloadAttributes] derivation
/// process.
///
/// It contains an inner [OraclePipeline] that is used to derive the attributes, backed by
/// oracle-based data sources.
///
/// [L2PayloadAttributes]: kona_derive::types::L2PayloadAttributes
#[derive(Debug)]
pub struct DerivationDriver<O: CommsClient + Send + Sync + Debug> {
    /// The current L2 safe head.
    l2_safe_head: L2BlockInfo,
    /// The header of the L2 safe head.
    l2_safe_head_header: Sealed<Header>,
    /// The inner pipeline.
    pipeline: OraclePipeline<O>,
}

impl<O: CommsClient + Send + Sync + Debug> DerivationDriver<O> {
    /// Returns the current L2 safe head [L2BlockInfo].
    pub fn l2_safe_head(&self) -> &L2BlockInfo {
        &self.l2_safe_head
    }

    /// Returns the [Header] of the current L2 safe head.
    pub fn l2_safe_head_header(&self) -> &Sealed<Header> {
        &self.l2_safe_head_header
    }

    /// Consumes self and returns the owned [Header] of the current L2 safe head.
    pub fn take_l2_safe_head_header(self) -> Sealed<Header> {
        self.l2_safe_head_header
    }

    /// Creates a new [DerivationDriver] with the given configuration, blob provider, and chain
    /// providers.
    ///
    /// ## Takes
    /// - `cfg`: The rollup configuration.
    /// - `blob_provider`: The blob provider.
    /// - `chain_provider`: The L1 chain provider.
    /// - `l2_chain_provider`: The L2 chain provider.
    ///
    /// ## Returns
    /// - A new [DerivationDriver] instance.
    pub async fn new(
        boot_info: &BootInfo,
        caching_oracle: &O,
        blob_provider: OracleBlobProvider<O>,
        mut chain_provider: OracleL1ChainProvider<O>,
        mut l2_chain_provider: OracleL2ChainProvider<O>,
    ) -> Result<Self> {
        let cfg = Arc::new(boot_info.rollup_config.clone());

        // Fetch the startup information.
        let (l1_origin, l2_safe_head, l2_safe_head_header) = Self::find_startup_info(
            caching_oracle,
            boot_info,
            &mut chain_provider,
            &mut l2_chain_provider,
        )
        .await?;

        // Construct the pipeline.
        let attributes = StatefulAttributesBuilder::new(
            cfg.clone(),
            l2_chain_provider.clone(),
            chain_provider.clone(),
        );
        let dap = EthereumDataSource::new(chain_provider.clone(), blob_provider, &cfg);
        let pipeline = PipelineBuilder::new()
            .rollup_config(cfg)
            .dap_source(dap)
            .l2_chain_provider(l2_chain_provider)
            .chain_provider(chain_provider)
            .builder(attributes)
            .origin(l1_origin)
            .build();

        Ok(Self { l2_safe_head, l2_safe_head_header, pipeline })
    }

    /// Produces the disputed [L2AttributesWithParent] payload, directly after the starting L2
    /// output root passed through the [BootInfo].
    pub async fn produce_disputed_payload(&mut self) -> Result<L2AttributesWithParent> {
        // As we start the safe head at the disputed block's parent, we step the pipeline until the
        // first attributes are produced. All batches at and before the safe head will be
        // dropped, so the first payload will always be the disputed one.
        let mut attributes = None;
        while attributes.is_none() {
            match self.pipeline.step(self.l2_safe_head).await {
                StepResult::PreparedAttributes => {
                    info!(target: "client_derivation_driver", "Stepped derivation pipeline")
                }
                StepResult::AdvancedOrigin => {
                    info!(target: "client_derivation_driver", "Advanced origin")
                }
                StepResult::OriginAdvanceErr(e) => {
                    warn!(target: "client_derivation_driver", "Failed to advance origin: {:?}", e)
                }
                StepResult::StepFailed(e) => {
                    warn!(target: "client_derivation_driver", "Failed to step derivation pipeline: {:?}", e)
                }
            }

            attributes = self.pipeline.next();
        }

        Ok(attributes.expect("Must be some"))
    }

    /// Finds the startup information for the derivation pipeline.
    ///
    /// ## Takes
    /// - `caching_oracle`: The caching oracle.
    /// - `boot_info`: The boot information.
    /// - `chain_provider`: The L1 chain provider.
    /// - `l2_chain_provider`: The L2 chain provider.
    ///
    /// ## Returns
    /// - A tuple containing the L1 origin block information and the L2 safe head information.
    async fn find_startup_info(
        caching_oracle: &O,
        boot_info: &BootInfo,
        chain_provider: &mut OracleL1ChainProvider<O>,
        l2_chain_provider: &mut OracleL2ChainProvider<O>,
    ) -> Result<(BlockInfo, L2BlockInfo, Sealed<Header>)> {
        // Find the initial safe head, based off of the starting L2 block number in the boot info.
        caching_oracle
            .write(&HintType::StartingL2Output.encode_with(&[boot_info.l2_output_root.as_ref()]))
            .await?;
        let mut output_preimage = [0u8; 128];
        caching_oracle
            .get_exact(
                PreimageKey::new(*boot_info.l2_output_root, PreimageKeyType::Keccak256),
                &mut output_preimage,
            )
            .await?;

        let safe_hash =
            output_preimage[96..128].try_into().map_err(|_| anyhow!("Invalid L2 output root"))?;
        let safe_header = l2_chain_provider.header_by_hash(safe_hash)?;
        let safe_head_info = l2_chain_provider.l2_block_info_by_number(safe_header.number).await?;

        let l1_origin =
            chain_provider.block_info_by_number(safe_head_info.l1_origin.number).await?;

        Ok((l1_origin, safe_head_info, Sealed::new_unchecked(safe_header, safe_hash)))
    }
}
