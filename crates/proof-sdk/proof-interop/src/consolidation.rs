//! Interop dependency resolution and consolidation logic.

use crate::{OptimisticBlock, OracleInteropProvider, PreState};
use alloc::{boxed::Box, vec::Vec};
use alloy_consensus::{Header, Sealed};
use alloy_primitives::{Bytes, Sealable};
use alloy_rpc_types_engine::PayloadAttributes;
use kona_executor::{NoopTrieDBProvider, StatelessL2BlockExecutor};
use kona_interop::{InteropProvider, MessageGraph, MessageGraphError};
use kona_mpt::NoopTrieHinter;
use kona_preimage::CommsClient;
use kona_proof::errors::OracleProviderError;
use maili_genesis::RollupConfig;
use op_alloy_consensus::OpTxType;
use op_alloy_rpc_types_engine::OpPayloadAttributes;
use tracing::{error, info};

/// The [SuperchainConsolidator] holds a [MessageGraph] and is responsible for recursively consolidating the
/// blocks within the graph, per [message validity rules].
///
/// [message validity rules]: https://specs.optimism.io/interop/messaging.html#invalid-messages
#[derive(Debug)]
pub struct SuperchainConsolidator<'a, C>
where
    C: CommsClient,
{
    /// The [PreState] being operated on.
    pre_state: &'a mut PreState,
    /// The [InteropProvider] used for the message graph.
    provider: OracleInteropProvider<C>,
    /// The [Header]s and their respective chain IDs to consolidate.
    headers: Vec<(u64, Sealed<Header>)>,
}

impl<'a, C> SuperchainConsolidator<'a, C>
where
    C: CommsClient + Send + Sync,
{
    /// Creates a new [MessageConsolidator] with the given [InteropProvider] and [Header]s.
    pub fn new(
        pre_state: &'a mut PreState,
        provider: OracleInteropProvider<C>,
        headers: Vec<(u64, Sealed<Header>)>,
    ) -> Self {
        Self { pre_state, provider, headers }
    }

    /// Consolidates the [Header]s within the [MessageGraph].
    ///
    /// This method will recursively consolidate the blocks within the [MessageGraph] until all invalid
    /// messages have been resolved.
    pub async fn consolidate(&mut self) -> Result<(), MessageGraphError<OracleProviderError>> {
        info!(target: "superchain_consolidator", "Consolidating superchain");

        match self.consolidate_once().await {
            Ok(()) => {
                info!(target: "superchain_consolidator", "Superchain consolidation complete");
                Ok(())
            }
            Err(MessageGraphError::InvalidMessages(_)) => Box::pin(self.consolidate()).await,
            Err(e) => {
                error!(target: "superchain_consolidator", "Error consolidating superchain: {:?}", e);
                Err(e)
            }
        }
    }

    /// Performs a single iteration of the consolidation process.
    ///
    /// Step-wise:
    /// 1. Derive a new [MessageGraph] from the current set of [Header]s.
    /// 2. Resolve the [MessageGraph].
    /// 3. If any invalid messages are found, re-execute the bad block(s) only deposit transactions, and bubble
    ///    up the error.
    async fn consolidate_once(&mut self) -> Result<(), MessageGraphError<OracleProviderError>> {
        // Derive the message graph from the current set of block headers.
        let graph = MessageGraph::derive(self.headers.as_slice(), self.provider.clone()).await?;

        // Attempt to resolve the message graph. If there were any invalid messages found, we must initiate
        // a re-execution of the original block, with only deposit transactions.
        if let Err(MessageGraphError::InvalidMessages(chain_ids)) = graph.resolve().await {
            self.re_execute_deposit_only(&chain_ids).await?;
            return Err(MessageGraphError::InvalidMessages(chain_ids));
        }

        Ok(())
    }

    /// Re-executes the original blocks, keyed by their chain IDs, with only their deposit transactions.
    async fn re_execute_deposit_only(
        &mut self,
        chain_ids: &[u64],
    ) -> Result<(), MessageGraphError<OracleProviderError>> {
        for chain_id in chain_ids {
            // Find the optimistic block header for the chain ID.
            let header = self
                .headers
                .iter_mut()
                .find(|(id, _)| id == chain_id)
                .map(|(_, header)| header)
                .ok_or(MessageGraphError::EmptyDependencySet)?;

            // Look up the parent header for the block.
            let parent_header = self.provider.header_by_hash(*chain_id, header.parent_hash).await?;

            // Re-derive the payload for the optimistic block to fetch its transactions.
            let transactions: Vec<Bytes> = self
                .provider
                .raw_non_canonical_transactions(*chain_id, header.transactions_root, header.number)
                .await?;

            // Explicitly panic if a block sent off for re-execution already contains nothing but deposits.
            assert!(
                !transactions.iter().all(|f| f.len() > 0 && f[0] == OpTxType::Deposit),
                "Impossible case; Block with only deposits found to be invalid."
            );

            // Re-craft the execution payload, trimming off all non-deposit transactions.
            let deposit_only_payload = OpPayloadAttributes {
                payload_attributes: PayloadAttributes {
                    timestamp: header.timestamp,
                    prev_randao: header.mix_hash,
                    suggested_fee_recipient: header.beneficiary,
                    withdrawals: Default::default(),
                    parent_beacon_block_root: header.parent_beacon_block_root,
                },
                transactions: Some(
                    transactions
                        .into_iter()
                        .filter(|t| t.len() > 0 && t[0] == OpTxType::Deposit as u8)
                        .collect(),
                ),
                no_tx_pool: Some(true),
                gas_limit: Some(header.gas_limit),
                eip_1559_params: Some(header.extra_data[1..].try_into().unwrap()),
            };

            // TODO: Send hint with chain ID + encoded payload to populate the key value store in the host with the
            // preimages required to re-execute the block. The host will be able to apply the payload onto the parent
            // state no problem.

            // TODO: Fetch the rollup config
            let rollup_config = RollupConfig::default();

            // Create a new stateless L2 block executor.
            //
            // TODO: We need to pass in actual implementations of the oracle-backed providers.
            let mut executor = StatelessL2BlockExecutor::builder(
                &rollup_config,
                NoopTrieDBProvider,
                NoopTrieHinter,
            )
            .with_parent_header(parent_header.seal_slow())
            .build();

            // Execute the block and take the new header.
            let new_header =
                executor.execute_payload(deposit_only_payload).unwrap().clone().seal_slow();
            let new_output_root = executor.compute_output_root().unwrap();

            // Replace the original optimistic block with the deposit only block.
            let PreState::TransitionState(ref mut transition_state) = self.pre_state else {
                panic!("SuperchainConsolidator received invalid PreState variant");
            };
            let original_optimistic_block = transition_state
                .pending_progress
                .iter_mut()
                .find(|block| block.block_hash == header.hash())
                .ok_or(MessageGraphError::EmptyDependencySet)?;
            *original_optimistic_block = OptimisticBlock::new(new_header.hash(), new_output_root);

            // Replace the original header with the new header.
            *header = new_header;
        }

        Ok(())
    }
}
