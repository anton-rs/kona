//! Contains the accelerated version of the KZG point evaluation precompile.

use alloc::{string::ToString, vec::Vec};
use alloy_primitives::{keccak256, Address, Bytes};
use anyhow::ensure;
use kona_preimage::{HintWriterClient, PreimageKey, PreimageKeyType, PreimageOracleClient};
use revm::{
    precompile::{u64_to_address, Error as PrecompileError, PrecompileWithAddress},
    primitives::{Precompile, PrecompileOutput, PrecompileResult},
};

use crate::{HintType, HINT_WRITER, ORACLE_READER};

const POINT_EVAL_ADDRESS: Address = u64_to_address(0x0A);

pub(crate) const FPVM_KZG_POINT_EVAL: PrecompileWithAddress =
    PrecompileWithAddress(POINT_EVAL_ADDRESS, Precompile::Standard(fpvm_kzg_point_eval));

/// Performs an FPVM-accelerated KZG point evaluation precompile call.
fn fpvm_kzg_point_eval(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    const GAS_COST: u64 = 50_000;

    if gas_limit < GAS_COST {
        return Err(PrecompileError::OutOfGas.into());
    }

    if input.len() != 192 {
        return Err(PrecompileError::BlobInvalidInputLength.into());
    }

    let result_data = kona_common::block_on(async move {
        // Write the hint for the ecrecover precompile run.
        let hint_data = &[POINT_EVAL_ADDRESS.as_ref(), input.as_ref()];
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

    Ok(PrecompileOutput::new(GAS_COST, result_data.into()))
}
