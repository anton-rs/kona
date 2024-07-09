//! Contains the L2-specifc contstructs of the client program.

mod chain_provider;
pub use chain_provider::OracleL2ChainProvider;

mod precompiles;
pub use precompiles::FPVMPrecompileOverride;
