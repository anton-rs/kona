//! Utilities for the interop proof program

use alloc::string::ToString;
use alloy_primitives::{Bytes, B256};
use kona_preimage::{errors::PreimageOracleError, CommsClient, PreimageKey, PreimageKeyType};
use kona_proof::errors::OracleProviderError;
use kona_proof_interop::{BootInfo, HintType, PreState};

/// Reads the raw pre-state from the preimage oracle.
pub(crate) async fn read_raw_pre_state<O>(
    caching_oracle: &O,
    boot_info: &BootInfo,
) -> Result<Bytes, OracleProviderError>
where
    O: CommsClient,
{
    caching_oracle
        .write(&HintType::AgreedPreState.encode_with(&[boot_info.agreed_pre_state.as_ref()]))
        .await
        .map_err(OracleProviderError::Preimage)?;
    let pre = caching_oracle
        .get(PreimageKey::new(*boot_info.agreed_pre_state, PreimageKeyType::Keccak256))
        .await
        .map_err(OracleProviderError::Preimage)?;

    if pre.is_empty() {
        return Err(OracleProviderError::Preimage(PreimageOracleError::Other(
            "Invalid pre-state preimage".to_string(),
        )));
    }

    Ok(Bytes::from(pre))
}

/// Fetches the safe head hash of the L2 chain based on the agreed upon L2 output root in the
/// [BootInfo].
pub(crate) async fn fetch_l2_safe_head_hash<O>(
    caching_oracle: &O,
    pre: &PreState,
) -> Result<B256, OracleProviderError>
where
    O: CommsClient,
{
    // Fetch the output root of the safe head block for the current L2 chain.
    let rich_output = match pre {
        PreState::SuperRoot(super_root) => {
            super_root.output_roots.first().ok_or(OracleProviderError::Preimage(
                PreimageOracleError::Other("No output roots in super root".to_string()),
            ))?
        }
        PreState::TransitionState(transition_state) => {
            transition_state.pre_state.output_roots.get(transition_state.step as usize).ok_or(
                OracleProviderError::Preimage(PreimageOracleError::Other(
                    "No output roots in transition state's pending progress".to_string(),
                )),
            )?
        }
    };

    caching_oracle
        .write(&HintType::L2OutputRoot.encode_with(&[
            rich_output.output_root.as_slice(),
            rich_output.chain_id.to_be_bytes().as_slice(),
        ]))
        .await
        .map_err(OracleProviderError::Preimage)?;
    let output_preimage = caching_oracle
        .get(PreimageKey::new(*rich_output.output_root, PreimageKeyType::Keccak256))
        .await
        .map_err(OracleProviderError::Preimage)?;

    output_preimage[96..128].try_into().map_err(OracleProviderError::SliceConversion)
}
