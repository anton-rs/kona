//! Contains the [KonaHandleRegister] function for registering the FPVM-accelerated precompiles.
//!
//! [KonaHandleRegister]: kona_executor::KonaHandleRegister

use alloc::sync::Arc;
use kona_mpt::{TrieDB, TrieDBFetcher, TrieDBHinter};
use revm::{
    handler::register::EvmHandler, precompile::PrecompileSpecId, primitives::SpecId,
    ContextPrecompiles, State,
};

mod bn128_pair;
mod ecrecover;
mod kzg_point_eval;

/// The [KonaHandleRegister] function for registering the FPVM-accelerated precompiles.
///
/// [KonaHandleRegister]: kona_executor::KonaHandleRegister
pub(crate) fn fpvm_handle_register<F, H>(
    handler: &mut EvmHandler<'_, (), &mut State<&mut TrieDB<F, H>>>,
) where
    F: TrieDBFetcher,
    H: TrieDBHinter,
{
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

        // Ensure the secp256r1 P256verify precompile is enabled in the FJORD spec
        if spec_id.is_enabled_in(SpecId::FJORD) {
            ctx_precompiles.extend([
                // EIP-7212: secp256r1 P256verify
                revm::precompile::secp256r1::P256VERIFY,
            ]);
        }

        ctx_precompiles
    });
}
