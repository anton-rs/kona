//! Contains the accelerated version of the `ecPairing` precompile.

use alloc::{string::ToString, sync::Arc, vec::Vec};
use alloy_primitives::{keccak256, Address, Bytes};
use kona_preimage::{errors::PreimageOracleError, CommsClient, PreimageKey, PreimageKeyType};
use kona_proof::{errors::OracleProviderError, HintType};
use revm::{
    precompile::{
        bn128::pair::{ISTANBUL_PAIR_BASE, ISTANBUL_PAIR_PER_POINT},
        u64_to_address, Error as PrecompileError,
    },
    primitives::{PrecompileOutput, PrecompileResult, StatefulPrecompile},
};

/// The address of the `ecPairing` precompile.
pub const ECPAIRING_ADDRESS: Address = u64_to_address(8);

/// The length of a single pair element.
const PAIR_ELEMENT_LEN: usize = 64 + 128;

/// An accelerated version of the `ecPairing` precompile that calls out to the host for the
/// result of the precompile execution.
#[derive(Debug)]
pub struct EcPairingAccelerated<C>
where
    C: CommsClient,
{
    /// The comms client.
    comms_client: Arc<C>,
}

impl<C> EcPairingAccelerated<C>
where
    C: CommsClient,
{
    /// Creates a new [EcPairingAccelerated] instance.
    pub fn new(comms_client: Arc<C>) -> Self {
        Self { comms_client }
    }
}

impl<C> StatefulPrecompile for EcPairingAccelerated<C>
where
    C: CommsClient + Send + Sync,
{
    fn call(&self, input: &Bytes, gas_limit: u64, _: &revm::primitives::Env) -> PrecompileResult {
        fpvm_ecpairing(self.comms_client.as_ref(), input, gas_limit)
    }
}

/// An accelerated version of the `ecPairing` precompile that calls out to the host for the
/// result of the precompile execution after the Granite hardfork.
#[derive(Debug)]
pub struct EcPairingAcceleratedGranite<C> {
    /// The comms client.
    comms_client: Arc<C>,
}

impl<C> EcPairingAcceleratedGranite<C> {
    /// Creates a new [EcPairingAcceleratedGranite] instance.
    pub fn new(comms_client: Arc<C>) -> Self {
        Self { comms_client }
    }
}

impl<C> StatefulPrecompile for EcPairingAcceleratedGranite<C>
where
    C: CommsClient + Send + Sync,
{
    fn call(&self, input: &Bytes, gas_limit: u64, _: &revm::primitives::Env) -> PrecompileResult {
        fpvm_ecpairing_granite(self.comms_client.as_ref(), input, gas_limit)
    }
}

/// Performs an FPVM-accelerated `ecpairing` precompile call.
fn fpvm_ecpairing<C>(comms_client: &C, input: &Bytes, gas_limit: u64) -> PrecompileResult
where
    C: CommsClient,
{
    let gas_used =
        (input.len() / PAIR_ELEMENT_LEN) as u64 * ISTANBUL_PAIR_PER_POINT + ISTANBUL_PAIR_BASE;

    if gas_used > gas_limit {
        return Err(PrecompileError::OutOfGas.into());
    }

    if input.len() % PAIR_ELEMENT_LEN != 0 {
        return Err(PrecompileError::Bn128PairLength.into());
    }

    let result_data = kona_proof::block_on(async move {
        // Write the hint for the ecrecover precompile run.
        let hint_data = &[ECPAIRING_ADDRESS.as_ref(), input.as_ref()];
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

    Ok(PrecompileOutput::new(gas_used, result_data.into()))
}

/// Performs an FPVM-accelerated `ecpairing` precompile call after the Granite hardfork.
fn fpvm_ecpairing_granite<C>(comms_client: &C, input: &Bytes, gas_limit: u64) -> PrecompileResult
where
    C: CommsClient,
{
    const BN256_MAX_PAIRING_SIZE_GRANITE: usize = 112_687;
    if input.len() > BN256_MAX_PAIRING_SIZE_GRANITE {
        return Err(PrecompileError::Bn128PairLength.into());
    }

    fpvm_ecpairing(comms_client, input, gas_limit)
}
