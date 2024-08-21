//! Contains the accelerated version of the `ecrecover` precompile.

use crate::fault::{HINT_WRITER, ORACLE_READER};
use alloc::{string::ToString, vec::Vec};
use alloy_primitives::{keccak256, Address, Bytes};
use anyhow::ensure;
use kona_client::HintType;
use kona_preimage::{HintWriterClient, PreimageKey, PreimageKeyType, PreimageOracleClient};
use revm::{
    precompile::{u64_to_address, Error as PrecompileError, PrecompileWithAddress},
    primitives::{Precompile, PrecompileOutput, PrecompileResult},
};

const ECRECOVER_ADDRESS: Address = u64_to_address(1);

pub(crate) const FPVM_ECRECOVER: PrecompileWithAddress =
    PrecompileWithAddress(ECRECOVER_ADDRESS, Precompile::Standard(fpvm_ecrecover));

/// Performs an FPVM-accelerated `ecrecover` precompile call.
fn fpvm_ecrecover(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    const ECRECOVER_BASE: u64 = 3_000;

    if ECRECOVER_BASE > gas_limit {
        return Err(PrecompileError::OutOfGas.into());
    }

    let result_data = kona_common::block_on(async move {
        // Write the hint for the ecrecover precompile run.
        let hint_data = &[ECRECOVER_ADDRESS.as_ref(), input.as_ref()];
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

    Ok(PrecompileOutput::new(ECRECOVER_BASE, result_data.into()))
}
