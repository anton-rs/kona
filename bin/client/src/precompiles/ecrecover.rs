//! Contains the accelerated version of the `ecrecover` precompile.

use alloc::{string::ToString, sync::Arc, vec::Vec};
use alloy_primitives::{keccak256, Address, Bytes};
use kona_preimage::{errors::PreimageOracleError, CommsClient, PreimageKey, PreimageKeyType};
use kona_proof::{errors::OracleProviderError, HintType};
use revm::{
    precompile::{u64_to_address, Error as PrecompileError},
    primitives::{PrecompileOutput, PrecompileResult, StatefulPrecompile},
};

/// The address of the `ecrecover` precompile.
pub const ECRECOVER_ADDRESS: Address = u64_to_address(1);

/// An accelerated version of the `ecrecover` precompile that calls out to the host for the
/// result of the precompile execution
#[derive(Debug)]
pub struct EcRecoverAccelerated<C>
where
    C: CommsClient,
{
    /// The comms client.
    comms_client: Arc<C>,
}

impl<C> EcRecoverAccelerated<C>
where
    C: CommsClient,
{
    /// Creates a new [EcRecoverAccelerated] instance.
    pub fn new(comms_client: Arc<C>) -> Self {
        Self { comms_client }
    }
}

impl<C> StatefulPrecompile for EcRecoverAccelerated<C>
where
    C: CommsClient + Send + Sync,
{
    fn call(&self, input: &Bytes, gas_limit: u64, _: &revm::primitives::Env) -> PrecompileResult {
        fpvm_ecrecover(self.comms_client.as_ref(), input, gas_limit)
    }
}

/// Performs an FPVM-accelerated `ecrecover` precompile call.
fn fpvm_ecrecover<C>(comms_client: &C, input: &Bytes, gas_limit: u64) -> PrecompileResult
where
    C: CommsClient,
{
    const ECRECOVER_BASE: u64 = 3_000;

    if ECRECOVER_BASE > gas_limit {
        return Err(PrecompileError::OutOfGas.into());
    }

    let result_data = kona_proof::block_on(async move {
        // Write the hint for the ecrecover precompile run.
        let hint_data = &[ECRECOVER_ADDRESS.as_ref(), input.as_ref()];
        comms_client
            .write(&HintType::L1Precompile.encode_with(hint_data))
            .await
            .map_err(OracleProviderError::Preimage)?;

        // Construct the key hash for the ecrecover precompile run.
        let raw_key_data = hint_data.iter().copied().flatten().copied().collect::<Vec<u8>>();
        let key_hash = keccak256(&raw_key_data);

        // Fetch the result of the ecrecover precompile run from the host.
        let result_data = comms_client
            .get(PreimageKey::new(*key_hash, PreimageKeyType::Precompile))
            .await
            .map_err(OracleProviderError::Preimage)?;

        // Ensure we've received valid result data.
        if result_data.is_empty() {
            return Err(OracleProviderError::Preimage(PreimageOracleError::Other(
                "Invalid result data".to_string(),
            )));
        }

        // Ensure we've not received an error from the host.
        if result_data[0] == 0 {
            return Err(OracleProviderError::Preimage(PreimageOracleError::Other(
                "Error executing ecrecover precompile in host".to_string(),
            )));
        }

        // Return the result data.
        Ok(result_data[1..].to_vec())
    })
    .map_err(|e| PrecompileError::Other(e.to_string()))?;

    Ok(PrecompileOutput::new(ECRECOVER_BASE, result_data.into()))
}
