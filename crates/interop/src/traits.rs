//! Traits for the `kona-interop` crate.

use crate::errors::InteropProviderResult;
use alloc::{boxed::Box, vec::Vec};
use alloy_consensus::Header;
use alloy_primitives::B256;
use async_trait::async_trait;
use op_alloy_consensus::OpReceiptEnvelope;

/// Describes the interface of the interop data provider. This provider is multiplexed over several
/// chains, with each method consuming a chain ID to determine the target chain.
#[async_trait]
pub trait InteropProvider {
    /// Fetch a [Header] by its number.
    async fn header_by_number(&self, chain_id: u64, number: u64) -> InteropProviderResult<Header>;

    /// Fetch a [Header] by its hash.
    async fn header_by_hash(&self, chain_id: u64, hash: B256) -> InteropProviderResult<Header>;

    /// Fetch all receipts for a given block by number.
    async fn receipts_by_number(
        &self,
        chain_id: u64,
        number: u64,
    ) -> InteropProviderResult<Vec<OpReceiptEnvelope>>;

    /// Fetch all receipts for a given block by hash.
    async fn receipts_by_hash(
        &self,
        chain_id: u64,
        block_hash: B256,
    ) -> InteropProviderResult<Vec<OpReceiptEnvelope>>;
}
