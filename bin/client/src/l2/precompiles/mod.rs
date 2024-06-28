//! Contains the [PrecompileOverride] trait implementation for the FPVM-accelerated precompiles.

use alloc::sync::Arc;
use kona_executor::PrecompileOverride;
use kona_mpt::{TrieDB, TrieDBFetcher, TrieDBHinter};
use revm::{
    handler::register::EvmHandler, precompile::PrecompileSpecId, ContextPrecompiles, State,
};

mod bn128_pair;
mod ecrecover;

/// The [PrecompileOverride] implementation for the FPVM-accelerated precompiles.
#[derive(Debug)]
pub struct FPVMPrecompileOverride<F, H>
where
    F: TrieDBFetcher,
    H: TrieDBHinter,
{
    _phantom: core::marker::PhantomData<(F, H)>,
}

impl<F, H> Default for FPVMPrecompileOverride<F, H>
where
    F: TrieDBFetcher,
    H: TrieDBHinter,
{
    fn default() -> Self {
        Self { _phantom: core::marker::PhantomData::<(F, H)> }
    }
}

impl<F, H> PrecompileOverride<F, H> for FPVMPrecompileOverride<F, H>
where
    F: TrieDBFetcher,
    H: TrieDBHinter,
{
    fn set_precompiles(handler: &mut EvmHandler<'_, (), &mut State<TrieDB<F, H>>>) {
        let spec_id = handler.cfg.spec_id;

        handler.pre_execution.load_precompiles = Arc::new(move || {
            let mut ctx_precompiles =
                ContextPrecompiles::new(PrecompileSpecId::from_spec_id(spec_id)).clone();

            // Extend with FPVM-accelerated precompiles
            let override_precompiles = [ecrecover::FPVM_ECRECOVER, bn128_pair::FPVM_ECPAIRING];
            ctx_precompiles.extend(override_precompiles);

            ctx_precompiles
        });
    }
}
