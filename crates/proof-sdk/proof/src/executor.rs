//! An executor constructor.

use alloc::{boxed::Box, sync::Arc};
use alloy_consensus::{Header, Sealed};
use alloy_primitives::B256;
use async_trait::async_trait;
use kona_driver::Executor;
use kona_executor::{StatelessL2BlockExecutor, TrieDBProvider};
use op_alloy_consensus::OpTxEnvelope;
use reth_evm::ConfigureEvm;
use kona_mpt::TrieHinter;
use op_alloy_genesis::RollupConfig;
use op_alloy_rpc_types_engine::OpPayloadAttributes;

/// An executor wrapper type.
#[derive(Debug)]
pub struct KonaExecutor<'a, P, H, C>
where
    P: TrieDBProvider + Send + Sync + Clone,
    H: TrieHinter + Send + Sync + Clone,
    C: ConfigureEvm<Header=Header, Transaction=OpTxEnvelope>,
{
    /// The rollup config for the executor.
    rollup_config: &'a Arc<RollupConfig>,
    /// The trie provider for the executor.
    trie_provider: P,
    /// The trie hinter for the executor.
    trie_hinter: H,
    /// EVM config.
    evm_config: C,
    /// The executor.
    inner: Option<StatelessL2BlockExecutor<'a, P, H, C>>,
}

impl<'a, P, H, C> KonaExecutor<'a, P, H, C>
where
    P: TrieDBProvider + Send + Sync + Clone,
    H: TrieHinter + Send + Sync + Clone,
    C: ConfigureEvm<Header=Header, Transaction=OpTxEnvelope>,
{
    /// Creates a new executor.
    pub const fn new(
        rollup_config: &'a Arc<RollupConfig>,
        trie_provider: P,
        trie_hinter: H,
        evm_config: C,
        inner: Option<StatelessL2BlockExecutor<'a, P, H, C>>,
    ) -> Self {
        Self { rollup_config, trie_provider, trie_hinter, evm_config, inner }
    }
}

#[async_trait]
impl<P, H, C> Executor for KonaExecutor<'_, P, H, C>
where
    P: TrieDBProvider + Send + Sync + Clone,
    H: TrieHinter + Send + Sync + Clone,
    C: ConfigureEvm<Header=Header, Transaction=OpTxEnvelope>,
{
    type Error = kona_executor::ExecutorError;

    /// Waits for the executor to be ready.
    async fn wait_until_ready(&mut self) {
        /* no-op for the kona executor */
        /* This is used when an engine api is used instead of a stateless block executor */
    }

    /// Updates the safe header.
    ///
    /// Since the L2 block executor is stateless, on an update to the safe head,
    /// a new executor is created with the updated header.
    fn update_safe_head(&mut self, header: Sealed<Header>) {
        let builder = StatelessL2BlockExecutor::builder(
            self.rollup_config,
            self.trie_provider.clone(),
            self.trie_hinter.clone(),
        )
        .with_parent_header(header)
        .with_evm_config(self.evm_config.clone());

        self.inner = Some(builder.build());
    }

    /// Execute the given payload attributes.
    async fn execute_payload(
        &mut self,
        attributes: OpPayloadAttributes,
    ) -> Result<Header, Self::Error> {
        self.inner
            .as_mut()
            .map_or_else(
                || Err(kona_executor::ExecutorError::MissingExecutor),
                |e| e.execute_payload(attributes),
            )
            .cloned()
    }

    /// Computes the output root.
    fn compute_output_root(&mut self) -> Result<B256, Self::Error> {
        self.inner.as_mut().map_or_else(
            || Err(kona_executor::ExecutorError::MissingExecutor),
            |e| e.compute_output_root(),
        )
    }
}
