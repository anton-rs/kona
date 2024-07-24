//! Contains the [PrecompileOverride] trait.

use kona_mpt::{TrieDB, TrieDBFetcher, TrieDBHinter};
use revm::{db::State, handler::register::EvmHandler};

/// A trait for defining precompile overrides during execution.
pub trait PrecompileOverride<F, H>
where
    F: TrieDBFetcher,
    H: TrieDBHinter,
{
    /// Set the precompiles to use during execution.
    fn set_precompiles(handler: &mut EvmHandler<'_, (), &mut State<&mut TrieDB<F, H>>>);
}

/// Default implementation of [PrecompileOverride], which does not override any precompiles.
#[derive(Debug, Default)]
pub struct NoPrecompileOverride;

impl<F, H> PrecompileOverride<F, H> for NoPrecompileOverride
where
    F: TrieDBFetcher,
    H: TrieDBHinter,
{
    fn set_precompiles(_: &mut EvmHandler<'_, (), &mut State<&mut TrieDB<F, H>>>) {
        // Do nothing
    }
}
