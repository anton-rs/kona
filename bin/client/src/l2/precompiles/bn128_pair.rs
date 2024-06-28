//! Contains the accelerated version of the `ecPairing` precompile.

use alloc::{string::ToString, vec::Vec};
use alloy_primitives::{keccak256, Address, Bytes};
use anyhow::ensure;
use kona_preimage::{HintWriterClient, PreimageKey, PreimageKeyType, PreimageOracleClient};
use revm::{
    precompile::{
        bn128::pair::{ISTANBUL_PAIR_BASE, ISTANBUL_PAIR_PER_POINT},
        u64_to_address, Error as PrecompileError, PrecompileWithAddress,
    },
    primitives::{Precompile, PrecompileOutput, PrecompileResult},
};

use crate::{HintType, HINT_WRITER, ORACLE_READER};

const ECPAIRING_ADDRESS: Address = u64_to_address(8);
const PAIR_ELEMENT_LEN: usize = 64 + 128;

pub(crate) const FPVM_ECPAIRING: PrecompileWithAddress =
    PrecompileWithAddress(ECPAIRING_ADDRESS, Precompile::Standard(fpvm_ecpairing));

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
        HINT_WRITER.write(&HintType::L1Precompile.encode_with(hint_data)).await?;

        // Construct the key hash for the ecrecover precompile run.
        let raw_key_data = hint_data.iter().copied().flatten().copied().collect::<Vec<u8>>();
        let key_hash = keccak256(&raw_key_data);

        // Fetch the result of the ecrecover precompile run from the host.
        let result_data =
            ORACLE_READER.get(PreimageKey::new(*key_hash, PreimageKeyType::Precompile)).await?;

        // Ensure we've received valid result data.
        ensure!(!result_data.is_empty(), "Invalid result data");

        // Ensure we've not received an error from the host.
        ensure!(result_data[0] != 0, "Error executing ecrecover precompile in host");

        // Return the result data.
        Ok(result_data[1..].to_vec())
    })
    .map_err(|e| PrecompileError::Other(e.to_string()))?;

    Ok(PrecompileOutput::new(gas_used, result_data.into()))
}
