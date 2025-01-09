//! Accelerated precompile runner for the host program.

use alloy_primitives::{Address, Bytes};
use anyhow::{anyhow, Result};
use revm::{
    precompile::{self, PrecompileWithAddress},
    primitives::{Env, Precompile},
};

/// List of precompiles that are accelerated by the host program.
pub(crate) const ACCELERATED_PRECOMPILES: &[PrecompileWithAddress] = &[
    precompile::secp256k1::ECRECOVER,                   // ecRecover
    precompile::bn128::pair::ISTANBUL,                  // ecPairing
    precompile::kzg_point_evaluation::POINT_EVALUATION, // KZG point evaluation
];

/// Executes an accelerated precompile on [revm].
pub(crate) fn execute<T: Into<Bytes>>(address: Address, input: T) -> Result<Vec<u8>> {
    if let Some(precompile) =
        ACCELERATED_PRECOMPILES.iter().find(|precompile| precompile.0 == address)
    {
        match precompile.1 {
            Precompile::Standard(std_precompile) => {
                // Standard precompile execution - no access to environment required.
                let output = std_precompile(&input.into(), u64::MAX)
                    .map_err(|e| anyhow!("Failed precompile execution: {e}"))?;

                Ok(output.bytes.into())
            }
            Precompile::Env(env_precompile) => {
                // Use default environment for KZG point evaluation.
                let output = env_precompile(&input.into(), u64::MAX, &Env::default())
                    .map_err(|e| anyhow!("Failed precompile execution: {e}"))?;

                Ok(output.bytes.into())
            }
            _ => anyhow::bail!("Precompile not accelerated"),
        }
    } else {
        anyhow::bail!("Precompile not accelerated");
    }
}
