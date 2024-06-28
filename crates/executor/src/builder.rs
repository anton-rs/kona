//! Contains the builder pattern for the [StatelessL2BlockExecutor].

use crate::{PrecompileOverride, StatelessL2BlockExecutor};
use alloy_consensus::{Header, Sealable, Sealed};
use anyhow::Result;
use kona_derive::types::RollupConfig;
use kona_mpt::{TrieDB, TrieDBFetcher, TrieDBHinter};
use revm::StateBuilder;

/// The builder pattern for the [StatelessL2BlockExecutor].
#[derive(Debug)]
pub struct StatelessL2BlockExecutorBuilder<'a, F, H, PO>
where
    F: TrieDBFetcher,
    H: TrieDBHinter,
    PO: PrecompileOverride<F, H>,
{
    /// The [RollupConfig].
    config: &'a RollupConfig,
    /// The parent [Header] to begin execution from.
    parent_header: Option<Sealed<Header>>,
    /// The precompile overrides to use during execution.
    precompile_overrides: Option<PO>,
    /// The [TrieDBFetcher] to fetch the state trie preimages.
    fetcher: Option<F>,
    /// The [TrieDBHinter] to hint the state trie preimages.
    hinter: Option<H>,
}

impl<'a, F, H, PO> StatelessL2BlockExecutorBuilder<'a, F, H, PO>
where
    F: TrieDBFetcher,
    H: TrieDBHinter,
    PO: PrecompileOverride<F, H>,
{
    /// Instantiate a new builder with the given [RollupConfig].
    pub fn with_config(config: &'a RollupConfig) -> Self {
        Self {
            config,
            parent_header: None,
            precompile_overrides: None,
            fetcher: None,
            hinter: None,
        }
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

    /// Set the precompile overrides to use during execution.
    pub fn with_precompile_overrides(mut self, precompile_overrides: PO) -> Self {
        self.precompile_overrides = Some(precompile_overrides);
        self
    }

    /// Build the [StatelessL2BlockExecutor] from the builder configuration.
    pub fn build(self) -> Result<StatelessL2BlockExecutor<'a, F, H, PO>> {
        let fetcher = self.fetcher.ok_or(anyhow::anyhow!("Fetcher not set"))?;
        let hinter = self.hinter.ok_or(anyhow::anyhow!("Hinter not set"))?;
        let parent_header = self.parent_header.unwrap_or_else(|| {
            let default_header = Header::default();
            default_header.seal_slow()
        });

        let trie_db = TrieDB::new(parent_header.state_root, parent_header, fetcher, hinter);
        let state = StateBuilder::new_with_database(trie_db).with_bundle_update().build();

        Ok(StatelessL2BlockExecutor {
            config: self.config,
            state,
            _phantom: core::marker::PhantomData::<PO>,
        })
    }
}
