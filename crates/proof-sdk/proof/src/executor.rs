//! An executor constructor.

use alloc::sync::Arc;
use alloy_consensus::{Header, Sealed};
use alloy_primitives::B256;
use kona_driver::{Executor, ExecutorConstructor};
use kona_executor::{KonaHandleRegister, StatelessL2BlockExecutor, TrieDBProvider};
use kona_mpt::TrieHinter;
use op_alloy_genesis::RollupConfig;
use op_alloy_rpc_types_engine::OpPayloadAttributes;

/// An executor wrapper type.
#[derive(Debug)]
pub struct KonaExecutor<'a, P, H>(StatelessL2BlockExecutor<'a, P, H>)
where
    P: TrieDBProvider + Send + Sync + Clone,
    H: TrieHinter + Send + Sync + Clone;

impl<'a, P, H> KonaExecutor<'a, P, H>
where
    P: TrieDBProvider + Send + Sync + Clone,
    H: TrieHinter + Send + Sync + Clone,
{
    /// Creates a new executor.
    pub const fn new(executor: StatelessL2BlockExecutor<'a, P, H>) -> Self {
        Self(executor)
    }
}

impl<P, H> Executor for KonaExecutor<'_, P, H>
where
    P: TrieDBProvider + Send + Sync + Clone,
    H: TrieHinter + Send + Sync + Clone,
{
    type Error = kona_executor::ExecutorError;

    /// Execute the given payload attributes.
    fn execute_payload(&mut self, attributes: OpPayloadAttributes) -> Result<&Header, Self::Error> {
        self.0.execute_payload(attributes)
    }

    /// Computes the output root.
    fn compute_output_root(&mut self) -> Result<B256, Self::Error> {
        self.0.compute_output_root()
    }
}

/// An executor constructor.
#[derive(Debug)]
pub struct KonaExecutorConstructor<'a, P, H>
where
    P: TrieDBProvider + Send + Sync + Clone,
    H: TrieHinter + Send + Sync + Clone,
{
    /// The rollup config for the executor.
    rollup_config: &'a Arc<RollupConfig>,
    /// The trie provider for the executor.
    trie_provider: P,
    /// The trie hinter for the executor.
    trie_hinter: H,
    /// The handle register for the executor.
    handle_register: KonaHandleRegister<P, H>,
}

impl<'a, P, H> KonaExecutorConstructor<'a, P, H>
where
    P: TrieDBProvider + Send + Sync + Clone,
    H: TrieHinter + Send + Sync + Clone,
{
    /// Creates a new executor constructor.
    pub fn new(
        rollup_config: &'a Arc<RollupConfig>,
        trie_provider: P,
        trie_hinter: H,
        handle_register: KonaHandleRegister<P, H>,
    ) -> Self {
        Self { rollup_config, trie_provider, trie_hinter, handle_register }
    }
}

impl<'a, P, H> ExecutorConstructor<KonaExecutor<'a, P, H>> for KonaExecutorConstructor<'a, P, H>
where
    P: TrieDBProvider + Send + Sync + Clone,
    H: TrieHinter + Send + Sync + Clone,
{
    /// Constructs the executor.
    fn new_executor(&self, header: Sealed<Header>) -> KonaExecutor<'a, P, H> {
        KonaExecutor::new(
            StatelessL2BlockExecutor::builder(
                self.rollup_config,
                self.trie_provider.clone(),
                self.trie_hinter.clone(),
            )
            .with_parent_header(header)
            .with_handle_register(self.handle_register)
            .build(),
        )
    }
}
