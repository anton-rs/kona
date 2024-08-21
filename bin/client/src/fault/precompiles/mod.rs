//! Contains the [PrecompileOverride] trait implementation for the FPVM-accelerated precompiles.

use alloc::sync::Arc;
use kona_executor::PrecompileOverride;
use kona_mpt::{TrieDB, TrieDBFetcher, TrieDBHinter};
use revm::{
    handler::register::EvmHandler, precompile::PrecompileSpecId, primitives::SpecId,
    ContextPrecompiles, State,
};

mod bn128_pair;
mod ecrecover;
mod kzg_point_eval;

/// The [PrecompileOverride] implementation for the FPVM-accelerated precompiles.
#[derive(Debug)]
pub(crate) struct FPVMPrecompileOverride<F, H>
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
    fn set_precompiles(handler: &mut EvmHandler<'_, (), &mut State<&mut TrieDB<F, H>>>) {
        let spec_id = handler.cfg.spec_id;

        handler.pre_execution.load_precompiles = Arc::new(move || {
            let mut ctx_precompiles =
                ContextPrecompiles::new(PrecompileSpecId::from_spec_id(spec_id)).clone();

            // Extend with FPVM-accelerated precompiles
            let override_precompiles = [
                ecrecover::FPVM_ECRECOVER,
                bn128_pair::FPVM_ECPAIRING,
                kzg_point_eval::FPVM_KZG_POINT_EVAL,
            ];
            ctx_precompiles.extend(override_precompiles);

            if spec_id.is_enabled_in(SpecId::FJORD) {
                ctx_precompiles.extend([
                    // EIP-7212: secp256r1 P256verify
                    revm::precompile::secp256r1::P256VERIFY,
                ]);
            }

            ctx_precompiles
        });
    }
}
