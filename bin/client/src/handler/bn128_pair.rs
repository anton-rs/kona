//! Contains the accelerated version of the `ecPairing` precompile.

use crate::{HINT_WRITER, ORACLE_READER};
use alloc::{string::ToString, vec::Vec};
use alloy_primitives::{keccak256, Address, Bytes};
use kona_preimage::{
    errors::PreimageOracleError, HintWriterClient, PreimageKey, PreimageKeyType,
    PreimageOracleClient,
};
use kona_proof::{errors::OracleProviderError, HintType};
use revm::{
    precompile::{
        bn128::pair::{ISTANBUL_PAIR_BASE, ISTANBUL_PAIR_PER_POINT},
        u64_to_address, Error as PrecompileError, PrecompileWithAddress,
    },
    primitives::{Precompile, PrecompileOutput, PrecompileResult},
};

const ECPAIRING_ADDRESS: Address = u64_to_address(8);
const PAIR_ELEMENT_LEN: usize = 64 + 128;

pub(crate) const FPVM_ECPAIRING: PrecompileWithAddress =
    PrecompileWithAddress(ECPAIRING_ADDRESS, Precompile::Standard(fpvm_ecpairing));

pub(crate) const FPVM_ECPAIRING_GRANITE: PrecompileWithAddress =
    PrecompileWithAddress(ECPAIRING_ADDRESS, Precompile::Standard(fpvm_ecpairing_granite));

/// Performs an FPVM-accelerated `ecpairing` precompile call.
fn fpvm_ecpairing(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let gas_used =
        (input.len() / PAIR_ELEMENT_LEN) as u64 * ISTANBUL_PAIR_PER_POINT + ISTANBUL_PAIR_BASE;

    if gas_used > gas_limit {
        return Err(PrecompileError::OutOfGas.into());
    }

    if input.len() % PAIR_ELEMENT_LEN != 0 {
        return Err(PrecompileError::Bn128PairLength.into());
    }

    let result_data = kona_common::block_on(async move {
        // Write the hint for the ecrecover precompile run.
        let hint_data = &[ECPAIRING_ADDRESS.as_ref(), input.as_ref()];
        HINT_WRITER
            .write(&HintType::L1Precompile.encode_with(hint_data))
            .await
            .map_err(OracleProviderError::Preimage)?;

        // Construct the key hash for the ecrecover precompile run.
        let raw_key_data = hint_data.iter().copied().flatten().copied().collect::<Vec<u8>>();
        let key_hash = keccak256(&raw_key_data);

        // Fetch the result of the ecrecover precompile run from the host.
        let result_data = ORACLE_READER
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
fn fpvm_ecpairing_granite(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    const BN256_MAX_PAIRING_SIZE_GRANITE: usize = 112_687;
    if input.len() > BN256_MAX_PAIRING_SIZE_GRANITE {
        return Err(PrecompileError::Bn128PairLength.into());
    }

    fpvm_ecpairing(input, gas_limit)
}
