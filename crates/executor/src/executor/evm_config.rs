use reth_evm::{ConfigureEvm, ConfigureEvmEnv};
use reth_primitives_traits::BlockHeader;
use alloy_consensus::{Sealed, Header as AlloyHeader};
use crate::db::TrieDB;
use revm::handler::register::EvmHandler;
use revm_primitives::EnvWithHandlerCfg;
use revm::{Evm, Database, db::State};

/// A type alias for the [revm::handler::register::HandleRegister] for kona's block executor.
pub type KonaHandleRegister<F, H> =
    for<'i> fn(&mut EvmHandler<'i, (), &mut State<&mut TrieDB<F, H>>>);

pub trait KonaEvmConfig: ConfigureEvm
{
    type Header: BlockHeader + From<&Sealed<alloy_consensus::Header>>;

    fn new() -> Self;
    fn handler_register(&self) -> Option<KonaHandleRegister<F, H>>;
    // fn set_precompiles<EXT, TrieDB>(handler: &mut EvmHandler<'_, EXT, TrieDB>);
}

pub struct DefaultEVMConfig<F, H> {
    pub handler_register: Option<KonaHandleRegister<F, H>>,
}

impl ConfigureEvm for DefaultEVMConfig {
    // fn evm(&self, db: TrieDB) -> EvmConfig {
    //     EvmBuilder::default()
    //         .with_db(db)
    //         .optimism()
    //         // add additional precompiles
    //         .append_handler_register(Self::set_precompiles)
    //         .build()
    // }

    fn evm_with_env<DB: Database>(
        &self,
        db: DB,
        env: EnvWithHandlerCfg,
    ) -> Evm<'_, Self::DefaultExternalContext<'_>, DB> {
        let mut base = Evm::builder().with_db(&mut state).with_env_with_handler_cfg(env);

        // If a handler register is provided, append it to the base EVM.
        if let Some(handler) = self.handler_register {
            base = base.append_handler_register(handler);
        }

        base.build()
    }

    fn evm_with_inspector<DB, I>(&self, db: DB, inspector: I) -> Evm<'_, I, DB> {
        unimplemented!()
    }

    fn evm_with_env_and_inspector<DB, I>(
        &self,
        db: DB,
        env: EnvWithHandlerCfg,
        inspector: I,
    ) -> Evm<'_, I, DB> {
        unimplemented!()
    }

    fn default_external_context<'a>(&self) -> Self::DefaultExternalContext<'a> {
        unimplemented!()
    };
}

impl KonaEvmConfig for DefaultEVMConfig {
    type Header = AlloyHeader;

    fn new() -> Self {
        DefaultEVMConfig
    }

    fn set_precompiles<EXT, TrieDB>(handler: &mut EvmHandler<'_, EXT, TrieDB>) {
        // first we need the evm spec id, which determines the precompiles
        let spec_id = handler.cfg.spec_id;

        // install the precompiles
        handler.pre_execution.load_precompiles = Arc::new(move || {
            let mut loaded_precompiles: ContextPrecompiles<DB> =
                ContextPrecompiles::new(PrecompileSpecId::from_spec_id(spec_id));

            loaded_precompiles.extend(secp256r1::precompiles());

            loaded_precompiles
        });
    }
}
