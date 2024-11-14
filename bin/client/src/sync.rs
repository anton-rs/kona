//! Sync Start

use crate::{
    errors::OracleProviderError, l1::OracleL1ChainProvider, l2::OracleL2ChainProvider, BootInfo,
    FlushableCache, HintType,
};
use alloc::sync::Arc;
use alloy_consensus::Sealed;
use core::fmt::Debug;
use kona_derive::traits::ChainProvider;
use kona_driver::{PipelineCursor, TipCursor};
use kona_executor::TrieDBProvider;
use kona_preimage::{CommsClient, PreimageKey, PreimageKeyType};
use op_alloy_protocol::BatchValidationProvider;

/// Constructs a [`PipelineCursor`] from the caching oracle, boot info, and providers.
pub async fn new_pipeline_cursor<O>(
    caching_oracle: Arc<O>,
    boot_info: &BootInfo,
    chain_provider: &mut OracleL1ChainProvider<O>,
    l2_chain_provider: &mut OracleL2ChainProvider<O>,
) -> Result<PipelineCursor, OracleProviderError>
where
    O: CommsClient + FlushableCache + FlushableCache + Send + Sync + Debug,
{
    // Find the initial safe head, based off of the starting L2 block number in the boot info.
    caching_oracle
        .write(&HintType::StartingL2Output.encode_with(&[boot_info.agreed_l2_output_root.as_ref()]))
        .await
        .map_err(OracleProviderError::Preimage)?;
    let mut output_preimage = [0u8; 128];
    caching_oracle
        .get_exact(
            PreimageKey::new(*boot_info.agreed_l2_output_root, PreimageKeyType::Keccak256),
            &mut output_preimage,
        )
        .await
        .map_err(OracleProviderError::Preimage)?;

    let safe_hash =
        output_preimage[96..128].try_into().map_err(OracleProviderError::SliceConversion)?;
    let safe_header = l2_chain_provider.header_by_hash(safe_hash)?;
    let safe_head_info = l2_chain_provider.l2_block_info_by_number(safe_header.number).await?;
    let l1_origin = chain_provider.block_info_by_number(safe_head_info.l1_origin.number).await?;

    // Walk back the starting L1 block by `channel_timeout` to ensure that the full channel is
    // captured.
    let channel_timeout =
        boot_info.rollup_config.channel_timeout(safe_head_info.block_info.timestamp);
    let mut l1_origin_number = l1_origin.number.saturating_sub(channel_timeout);
    if l1_origin_number < boot_info.rollup_config.genesis.l1.number {
        l1_origin_number = boot_info.rollup_config.genesis.l1.number;
    }
    let origin = chain_provider.block_info_by_number(l1_origin_number).await?;

    // Construct the cursor.
    let safe_header = Sealed::new_unchecked(safe_header, safe_hash);
    let mut cursor = PipelineCursor::new(channel_timeout, origin);
    let tip = TipCursor::new(safe_head_info, safe_header, boot_info.agreed_l2_output_root);
    cursor.advance(origin, tip);
    Ok(cursor)
}
