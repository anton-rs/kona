//! Contains the builder pattern for the [StatelessL2BlockExecutor].

use super::{StatelessL2BlockExecutor, KonaEvmConfig};
use crate::db::{TrieDB, TrieDBProvider};
use alloy_consensus::{Header, Sealable, Sealed};
use kona_mpt::TrieHinter;
use op_alloy_genesis::RollupConfig;
use reth_optimism_chainspec::OpChainSpec;
use alloc::sync::Arc;
use revm::{db::State, handler::register::EvmHandler};

/// A type alias for the [revm::handler::register::HandleRegister] for kona's block executor.
pub type KonaHandleRegister<F, H> =
    for<'i> fn(&mut EvmHandler<'i, (), &mut State<&mut TrieDB<F, H>>>);

#[derive(Debug)]
enum EvmConfigOrChainSpec<C: KonaEvmConfig> {
    EvmConfig(C),
    ChainSpec(Arc<OpChainSpec>),
}

/// The builder pattern for the [StatelessL2BlockExecutor].
#[derive(Debug)]
pub struct StatelessL2BlockExecutorBuilder<'a, F, H, C>
where
    F: TrieDBProvider,
    H: TrieHinter,
    C: KonaEvmConfig,
{
    /// The [RollupConfig].
    config: &'a RollupConfig,
    /// The [TrieDBProvider] to fetch the state trie preimages.
    provider: F,
    /// The [TrieHinter] to hint the state trie preimages.
    hinter: H,
    /// The parent [Header] to begin execution from.
    parent_header: Option<Sealed<Header>>,
    /// The [KonaEvmConfig] or chainspec used to derive it.
    evm_config: Option<EvmConfigOrChainSpec<C>>,
    /// The [KonaHandleRegister] to use during execution.
    handler_register: Option<KonaHandleRegister<F, H>>,
}

impl<'a, F, H, C> StatelessL2BlockExecutorBuilder<'a, F, H, C>
where
    F: TrieDBProvider,
    H: TrieHinter,
    C: KonaEvmConfig,
{
    /// Instantiate a new builder with the given [RollupConfig].
    pub fn new(config: &'a RollupConfig, provider: F, hinter: H) -> Self {
        Self { config, provider, hinter, parent_header: None, evm_config: None, handler_register: None }
    }

    /// Set the [Header] to begin execution from.
    pub fn with_parent_header(mut self, parent_header: Sealed<Header>) -> Self {
        self.parent_header = Some(parent_header);
        self
    }

    /// Set the [KonaHandleRegister] for execution.
    pub fn with_handle_register(mut self, handler_register: KonaHandleRegister<F, H>) -> Self {
        self.handler_register = Some(handler_register);
        self
    }

    /// Set the [KonaHandleRegister] for execution.
    pub fn with_evm_config(mut self, evm_config: C) -> Self {
        self.evm_config = Some(EvmConfigOrChainSpec::EvmConfig(evm_config));
        self
    }

    /// Set the [KonaHandleRegister] for execution.
    pub fn with_chain_spec(mut self, chain_spec: Arc<OpChainSpec>) -> Self {
        self.evm_config = Some(EvmConfigOrChainSpec::ChainSpec(chain_spec));
        self
    }

    /// Build the [StatelessL2BlockExecutor] from the builder configuration.
    pub fn build(self) -> StatelessL2BlockExecutor<'a, F, H, C> {
        let parent_header = self.parent_header.unwrap_or_else(|| {
            let default_header = Header::default();
            default_header.seal_slow()
        });

        // ZTODO: error handling
        let evm_config = match self.evm_config.unwrap() {
            EvmConfigOrChainSpec::EvmConfig(config) => config,
            EvmConfigOrChainSpec::ChainSpec(chain_spec) => C::new(chain_spec)
        };

        let trie_db =
            TrieDB::new(parent_header.state_root, parent_header, self.provider, self.hinter);

        StatelessL2BlockExecutor {
            config: self.config,
            trie_db,
            evm_config,
            handler_register: self.handler_register,
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
