//! Contains the [KonaHandleRegister] function for registering the FPVM-accelerated precompiles.
//!
//! [KonaHandleRegister]: kona_executor::KonaHandleRegister

use alloc::sync::Arc;
use kona_mpt::{TrieDB, TrieHinter, TrieProvider};
use revm::{
    handler::register::EvmHandler,
    primitives::{spec_to_generic, SpecId},
    State,
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
    F: TrieProvider,
    H: TrieHinter,
{
    let spec_id = handler.cfg.spec_id;

    handler.pre_execution.load_precompiles = Arc::new(move || {
        let mut ctx_precompiles = spec_to_generic!(spec_id, {
            revm::optimism::load_precompiles::<SPEC, (), &mut State<&mut TrieDB<F, H>>>()
        });

        // Extend with FPVM-accelerated precompiles
        let override_precompiles = [
            ecrecover::FPVM_ECRECOVER,
            bn128_pair::FPVM_ECPAIRING,
            kzg_point_eval::FPVM_KZG_POINT_EVAL,
        ];
        ctx_precompiles.extend(override_precompiles);
        ctx_precompiles
    });
}
