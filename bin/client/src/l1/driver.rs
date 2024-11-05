//! Contains the [DerivationDriver] struct, which handles the [OpPayloadAttributes] derivation
//! process.
//!
//! [OpPayloadAttributes]: op_alloy_rpc_types_engine::OpPayloadAttributes

use super::OracleL1ChainProvider;
use crate::{
    errors::{DriverError, DriverResult, OracleProviderError},
    l2::OracleL2ChainProvider,
    BootInfo, FlushableCache, HintType,
};
use alloc::{sync::Arc, vec::Vec};
use alloy_consensus::{BlockBody, Header, Sealable, Sealed};
use alloy_primitives::B256;
use alloy_rlp::Decodable;
use core::fmt::Debug;
use kona_derive::{
    attributes::StatefulAttributesBuilder,
    errors::{PipelineError, PipelineErrorKind, ResetError},
    pipeline::{DerivationPipeline, PipelineBuilder},
    sources::EthereumDataSource,
    stages::{
        AttributesQueue, BatchProvider, BatchStream, ChannelProvider, ChannelReader, FrameQueue,
        L1Retrieval, L1Traversal,
    },
    traits::{
        BlobProvider, ChainProvider, L2ChainProvider, OriginProvider, Pipeline, SignalReceiver,
    },
    types::{ActivationSignal, PipelineResult, ResetSignal, Signal, StepResult},
};
use kona_executor::{KonaHandleRegister, StatelessL2BlockExecutor};
use kona_mpt::{TrieHinter, TrieProvider};
use kona_preimage::{CommsClient, PreimageKey, PreimageKeyType};
use op_alloy_consensus::{OpBlock, OpTxEnvelope, OpTxType};
use op_alloy_genesis::RollupConfig;
use op_alloy_protocol::{BatchValidationProvider, BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::OpAttributesWithParent;
use tracing::{error, info, warn};

/// An oracle-backed derivation pipeline.
pub type OraclePipeline<O, B> = DerivationPipeline<
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

/// The [DerivationDriver] struct is responsible for handling the [OpPayloadAttributes]
/// derivation process.
///
/// It contains an inner [OraclePipeline] that is used to derive the attributes, backed by
/// oracle-based data sources.
///
/// [OpPayloadAttributes]: op_alloy_rpc_types_engine::OpPayloadAttributes
#[derive(Debug)]
pub struct DerivationDriver<O, B>
where
    O: CommsClient + Send + Sync + Debug,
    B: BlobProvider + Send + Sync + Debug + Clone,
{
    /// The current L2 safe head.
    l2_safe_head: L2BlockInfo,
    /// The header of the L2 safe head.
    l2_safe_head_header: Sealed<Header>,
    /// The output root of the L2 safe head.
    l2_safe_head_output_root: B256,
    /// The target L2 block number.
    target_block_number: u64,
    /// The inner pipeline.
    pipeline: OraclePipeline<O, B>,
    /// The caching oracle.
    caching_oracle: Arc<O>,
}

impl<O, B> DerivationDriver<O, B>
where
    O: CommsClient + FlushableCache + Send + Sync + Debug,
    B: BlobProvider + Send + Sync + Debug + Clone,
{
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
        caching_oracle: &Arc<O>,
        blob_provider: B,
        mut chain_provider: OracleL1ChainProvider<O>,
        mut l2_chain_provider: OracleL2ChainProvider<O>,
    ) -> DriverResult<Self> {
        let cfg = Arc::new(boot_info.rollup_config.clone());

        // Fetch the startup information.
        let (l1_origin, l2_safe_head, l2_safe_head_header) = Self::sync_start(
            caching_oracle,
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

        Ok(Self {
            l2_safe_head,
            l2_safe_head_header,
            l2_safe_head_output_root: boot_info.agreed_l2_output_root,
            target_block_number: boot_info.claimed_l2_block_number,
            pipeline,
            caching_oracle: caching_oracle.clone(),
        })
    }

    /// Advances the derivation pipeline to the target block number.
    ///
    /// ## Takes
    /// - `cfg`: The rollup configuration.
    /// - `provider`: The trie provider.
    /// - `hinter`: The trie hinter.
    /// - `handle_register`: The handle register for the EVM.
    ///
    /// ## Returns
    /// - `Ok((number, output_root))` - A tuple containing the number of the produced block and the
    ///   output root.
    /// - `Err(e)` - An error if the block could not be produced.
    pub async fn advance_to_target<P, H>(
        &mut self,
        cfg: &RollupConfig,
        provider: &P,
        hinter: &H,
        handle_register: KonaHandleRegister<P, H>,
    ) -> DriverResult<(u64, B256)>
    where
        P: TrieProvider + Send + Sync + Clone,
        H: TrieHinter + Send + Sync + Clone,
    {
        loop {
            // Check if we have reached the target block number.
            if self.l2_safe_head().block_info.number >= self.target_block_number {
                info!(target: "client", "Derivation complete, reached L2 safe head.");
                return Ok((self.l2_safe_head.block_info.number, self.l2_safe_head_output_root));
            }

            let OpAttributesWithParent { mut attributes, .. } = match self.produce_payload().await {
                Ok(attrs) => attrs,
                Err(PipelineErrorKind::Critical(PipelineError::EndOfSource)) => {
                    warn!(target: "client", "Exhausted data source; Halting derivation and using current safe head.");

                    // Adjust the target block number to the current safe head, as no more blocks
                    // can be produced.
                    self.target_block_number = self.l2_safe_head.block_info.number;
                    continue;
                }
                Err(e) => {
                    error!(target: "client", "Failed to produce payload: {:?}", e);
                    return Err(e.into());
                }
            };

            let mut executor = self.new_executor(cfg, provider, hinter, handle_register);
            let header = match executor.execute_payload(attributes.clone()) {
                Ok(header) => header,
                Err(e) => {
                    error!(target: "client", "Failed to execute L2 block: {}", e);

                    if cfg.is_holocene_active(attributes.payload_attributes.timestamp) {
                        // Retry with a deposit-only block.
                        warn!(target: "client", "Flushing current channel and retrying deposit only block");

                        // Flush the current batch and channel - if a block was replaced with a
                        // deposit-only block due to execution failure, the
                        // batch and channel it is contained in is forwards
                        // invalidated.
                        self.pipeline.signal(Signal::FlushChannel).await?;

                        // Strip out all transactions that are not deposits.
                        attributes.transactions = attributes.transactions.map(|txs| {
                            txs.into_iter()
                                .filter(|tx| (!tx.is_empty() && tx[0] == OpTxType::Deposit as u8))
                                .collect::<Vec<_>>()
                        });

                        // Retry the execution.
                        executor = self.new_executor(cfg, provider, hinter, handle_register);
                        match executor.execute_payload(attributes.clone()) {
                            Ok(header) => header,
                            Err(e) => {
                                error!(
                                    target: "client",
                                    "Critical - Failed to execute deposit-only block: {e}",
                                );
                                return Err(e.into());
                            }
                        }
                    } else {
                        // Pre-Holocene, discard the block if execution fails.
                        continue;
                    }
                }
            };

            // Construct the block.
            let block = OpBlock {
                header: header.clone(),
                body: BlockBody {
                    transactions: attributes
                        .transactions
                        .unwrap_or_default()
                        .into_iter()
                        .map(|tx| OpTxEnvelope::decode(&mut tx.as_ref()).map_err(DriverError::Rlp))
                        .collect::<DriverResult<Vec<OpTxEnvelope>>>()?,
                    ommers: Vec::new(),
                    withdrawals: None,
                },
            };

            // Update the safe head.
            self.l2_safe_head =
                L2BlockInfo::from_block_and_genesis(&block, &self.pipeline.rollup_config.genesis)
                    .map_err(OracleProviderError::BlockInfo)?;
            self.l2_safe_head_header = header.clone().seal_slow();
            self.l2_safe_head_output_root = executor.compute_output_root()?;
        }
    }

    /// Produces the disputed [OpAttributesWithParent] payload, directly after the starting L2
    /// output root passed through the [BootInfo].
    async fn produce_payload(&mut self) -> PipelineResult<OpAttributesWithParent> {
        // As we start the safe head at the disputed block's parent, we step the pipeline until the
        // first attributes are produced. All batches at and before the safe head will be
        // dropped, so the first payload will always be the disputed one.
        loop {
            match self.pipeline.step(self.l2_safe_head).await {
                StepResult::PreparedAttributes => {
                    info!(target: "client_derivation_driver", "Stepped derivation pipeline")
                }
                StepResult::AdvancedOrigin => {
                    info!(target: "client_derivation_driver", "Advanced origin")
                }
                StepResult::OriginAdvanceErr(e) | StepResult::StepFailed(e) => {
                    warn!(target: "client_derivation_driver", "Failed to step derivation pipeline: {:?}", e);

                    // Break the loop unless the error signifies that there is not enough data to
                    // complete the current step. In this case, we retry the step to see if other
                    // stages can make progress.
                    match e {
                        PipelineErrorKind::Temporary(_) => continue,
                        PipelineErrorKind::Reset(e) => {
                            let system_config = self
                                .pipeline
                                .l2_chain_provider
                                .system_config_by_number(
                                    self.l2_safe_head.block_info.number,
                                    self.pipeline.rollup_config.clone(),
                                )
                                .await?;

                            if matches!(e, ResetError::HoloceneActivation) {
                                self.pipeline
                                    .signal(
                                        ActivationSignal {
                                            l2_safe_head: self.l2_safe_head,
                                            l1_origin: self
                                                .pipeline
                                                .origin()
                                                .ok_or(PipelineError::MissingOrigin.crit())?,
                                            system_config: Some(system_config),
                                        }
                                        .signal(),
                                    )
                                    .await?;
                            } else {
                                // Flush the caching oracle if a reorg is detected.
                                if matches!(e, ResetError::ReorgDetected(_, _)) {
                                    self.caching_oracle.as_ref().flush();
                                }

                                // Reset the pipeline to the initial L2 safe head and L1 origin,
                                // and try again.
                                self.pipeline
                                    .signal(
                                        ResetSignal {
                                            l2_safe_head: self.l2_safe_head,
                                            l1_origin: self
                                                .pipeline
                                                .origin()
                                                .ok_or(PipelineError::MissingOrigin.crit())?,
                                            system_config: Some(system_config),
                                        }
                                        .signal(),
                                    )
                                    .await?;
                            }
                        }
                        PipelineErrorKind::Critical(_) => return Err(e),
                    }
                }
            }

            if let Some(attrs) = self.pipeline.next() {
                return Ok(attrs);
            }
        }
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
    async fn sync_start(
        caching_oracle: &O,
        boot_info: &BootInfo,
        chain_provider: &mut OracleL1ChainProvider<O>,
        l2_chain_provider: &mut OracleL2ChainProvider<O>,
    ) -> DriverResult<(BlockInfo, L2BlockInfo, Sealed<Header>)> {
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

        Ok((l1_origin, safe_head_info, Sealed::new_unchecked(safe_header, safe_hash)))
    }

    /// Returns a new [StatelessL2BlockExecutor] instance.
    ///
    /// ## Takes
    /// - `cfg`: The rollup configuration.
    /// - `provider`: The trie provider.
    /// - `hinter`: The trie hinter.
    /// - `handle_register`: The handle register for the EVM.
    fn new_executor<'a, P, H>(
        &mut self,
        cfg: &'a RollupConfig,
        provider: &P,
        hinter: &H,
        handle_register: KonaHandleRegister<P, H>,
    ) -> StatelessL2BlockExecutor<'a, P, H>
    where
        P: TrieProvider + Send + Sync + Clone,
        H: TrieHinter + Send + Sync + Clone,
    {
        StatelessL2BlockExecutor::builder(cfg, provider.clone(), hinter.clone())
            .with_parent_header(self.l2_safe_head_header().clone())
            .with_handle_register(handle_register)
            .build()
    }
}
