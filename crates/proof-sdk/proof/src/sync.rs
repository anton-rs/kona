//! Sync Start

use crate::{
    errors::OracleProviderError, l1::OracleL1ChainProvider, l2::OracleL2ChainProvider, BootInfo,
    FlushableCache,
};
use alloy_consensus::{Header, Sealed};
use core::fmt::Debug;
use kona_derive::traits::ChainProvider;
use kona_driver::{PipelineCursor, TipCursor};
use kona_preimage::CommsClient;
use op_alloy_protocol::BatchValidationProvider;

/// Constructs a [`PipelineCursor`] from the caching oracle, boot info, and providers.
pub async fn new_pipeline_cursor<O>(
    boot_info: &BootInfo,
    safe_header: Sealed<Header>,
    chain_provider: &mut OracleL1ChainProvider<O>,
    l2_chain_provider: &mut OracleL2ChainProvider<O>,
) -> Result<PipelineCursor, OracleProviderError>
where
    O: CommsClient + FlushableCache + FlushableCache + Send + Sync + Debug,
{
    let safe_head_info = l2_chain_provider.l2_block_info_by_number(safe_header.number).await?;
    let l1_origin = chain_provider.block_info_by_number(safe_head_info.l1_origin.number).await?;

    info!(target: "client", "hehe l1_origin {:?}", l1_origin);

    // Walk back the starting L1 block by `channel_timeout` to ensure that the full channel is
    // captured.
    let channel_timeout =
        boot_info.rollup_config.channel_timeout(safe_head_info.block_info.timestamp);
    let mut l1_origin_number = l1_origin.number.saturating_sub(channel_timeout);
    if l1_origin_number < boot_info.rollup_config.genesis.l1.number {
        l1_origin_number = boot_info.rollup_config.genesis.l1.number;
    }
    let origin = chain_provider.block_info_by_number(l1_origin_number).await?;

    info!(target: "client", "origin {:?}", origin);

    // Construct the cursor.
    let mut cursor = PipelineCursor::new(channel_timeout, origin);
    let tip = TipCursor::new(safe_head_info, safe_header, boot_info.agreed_l2_output_root);
    cursor.advance(origin, tip);
    Ok(cursor)
}
