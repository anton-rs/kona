//! Contains the builder pattern for the [StatelessL2BlockExecutor].

use super::StatelessL2BlockExecutor;
use crate::{
    db::{TrieDB, TrieDBProvider},
    DefaultEVMConfig
};
use alloy_consensus::{Header, Sealable, Sealed};
use kona_mpt::TrieHinter;
use op_alloy_genesis::RollupConfig;
use revm::{db::State, handler::register::EvmHandler};
use reth_evm::ConfigureEvm;

/// The builder pattern for the [StatelessL2BlockExecutor].
pub struct StatelessL2BlockExecutorBuilder<'a, F, H, C>
where
    F: TrieDBProvider,
    H: TrieHinter,
    C: ConfigureEvm,
{
    /// The [RollupConfig].
    config: &'a RollupConfig,
    /// The [TrieDBProvider] to fetch the state trie preimages.
    provider: F,
    /// The [TrieHinter] to hint the state trie preimages.
    hinter: H,
    /// The parent [Header] to begin execution from.
    parent_header: Option<Sealed<Header>>,
    /// The [ConfigureEvm] used for execution.
    evm_config: Option<C>,
}

impl<'a, F, H, C> StatelessL2BlockExecutorBuilder<'a, F, H, C>
where
    F: TrieDBProvider,
    H: TrieHinter,
    C: ConfigureEvm,
{
    /// Instantiate a new builder with the given [RollupConfig].
    pub fn new(config: &'a RollupConfig, provider: F, hinter: H) -> Self {
        Self { config, provider, hinter, parent_header: None, evm_config: None }
    }

    /// Set the [Header] to begin execution from.
    pub fn with_parent_header(mut self, parent_header: Sealed<Header>) -> Self {
        self.parent_header = Some(parent_header);
        self
    }

    /// Set the [KonaHandleRegister] for execution.
    pub fn with_evm_config(mut self, evm_config: C) -> Self {
        self.evm_config = Some(evm_config);
        self
    }

    /// Build the [StatelessL2BlockExecutor] from the builder configuration.
    pub fn build(self) -> StatelessL2BlockExecutor<'a, F, H, C> {
        let parent_header = self.parent_header.unwrap_or_else(|| {
            let default_header = Header::default();
            default_header.seal_slow()
        });

        // TODO: think through default behavior, may need chain spec or something
        let evm_config = self.evm_config.unwrap_or_else(|| DefaultEVMConfig::new());

        let trie_db =
            TrieDB::new(parent_header.state_root, parent_header, self.provider, self.hinter);

        StatelessL2BlockExecutor {
            config: self.config,
            trie_db,
            evm_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoopTrieDBProvider;
    use kona_mpt::NoopTrieHinter;

    #[test]
    fn test_build_full() {
        let config = RollupConfig::default();
        let parent_header = Header::default().seal_slow();

        fn test_handler_register<F, H>(_: &mut EvmHandler<'_, (), &mut State<&mut TrieDB<F, H>>>)
        where
            F: TrieDBProvider,
            H: TrieHinter,
        {
        }

        let executor =
            StatelessL2BlockExecutorBuilder::new(&config, NoopTrieDBProvider, NoopTrieHinter)
                .with_handle_register(test_handler_register)
                .build();

        assert_eq!(*executor.config, config);
        assert_eq!(*executor.trie_db.parent_block_header(), parent_header);
    }
}
