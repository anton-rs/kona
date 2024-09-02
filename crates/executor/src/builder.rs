//! Contains the builder pattern for the [StatelessL2BlockExecutor].

use crate::StatelessL2BlockExecutor;
use alloy_consensus::{Header, Sealable, Sealed};
use anyhow::Result;
use kona_mpt::{TrieDB, TrieDBFetcher, TrieDBHinter};
use kona_primitives::RollupConfig;
use revm::{db::State, handler::register::EvmHandler};

/// A type alias for the [revm::handler::register::HandleRegister] for kona's block executor.
pub type KonaHandleRegister<F, H> =
    for<'i> fn(&mut EvmHandler<'i, (), &mut State<&mut TrieDB<F, H>>>);

/// The builder pattern for the [StatelessL2BlockExecutor].
#[derive(Debug)]
pub struct StatelessL2BlockExecutorBuilder<'a, F, H>
where
    F: TrieDBFetcher,
    H: TrieDBHinter,
{
    /// The [RollupConfig].
    config: &'a RollupConfig,
    /// The parent [Header] to begin execution from.
    parent_header: Option<Sealed<Header>>,
    /// The [KonaHandleRegister] to use during execution.
    handler_register: Option<KonaHandleRegister<F, H>>,
    /// The [TrieDBFetcher] to fetch the state trie preimages.
    fetcher: Option<F>,
    /// The [TrieDBHinter] to hint the state trie preimages.
    hinter: Option<H>,
}

impl<'a, F, H> StatelessL2BlockExecutorBuilder<'a, F, H>
where
    F: TrieDBFetcher,
    H: TrieDBHinter,
{
    /// Instantiate a new builder with the given [RollupConfig].
    pub fn with_config(config: &'a RollupConfig) -> Self {
        Self { config, parent_header: None, handler_register: None, fetcher: None, hinter: None }
    }

    /// Set the [Header] to begin execution from.
    pub fn with_parent_header(mut self, parent_header: Sealed<Header>) -> Self {
        self.parent_header = Some(parent_header);
        self
    }

    /// Set the [TrieDBFetcher] to fetch the state trie preimages.
    pub fn with_fetcher(mut self, fetcher: F) -> Self {
        self.fetcher = Some(fetcher);
        self
    }

    /// Set the [TrieDBHinter] to hint the state trie preimages.
    pub fn with_hinter(mut self, hinter: H) -> Self {
        self.hinter = Some(hinter);
        self
    }

    /// Set the [KonaHandleRegister] for execution.
    pub fn with_handle_register(mut self, handler_register: KonaHandleRegister<F, H>) -> Self {
        self.handler_register = Some(handler_register);
        self
    }

    /// Build the [StatelessL2BlockExecutor] from the builder configuration.
    pub fn build(self) -> Result<StatelessL2BlockExecutor<'a, F, H>> {
        let fetcher = self.fetcher.ok_or(anyhow::anyhow!("Fetcher not set"))?;
        let hinter = self.hinter.ok_or(anyhow::anyhow!("Hinter not set"))?;
        let parent_header = self.parent_header.unwrap_or_else(|| {
            let default_header = Header::default();
            default_header.seal_slow()
        });

        let trie_db = TrieDB::new(parent_header.state_root, parent_header, fetcher, hinter);
        Ok(StatelessL2BlockExecutor {
            config: self.config,
            trie_db,
            handler_register: self.handler_register,
        })
    }
}
