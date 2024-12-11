//! Traits for the `kona-interop` crate.

use alloc::vec::Vec;
use alloy_primitives::B256;
use async_trait::async_trait;
use op_alloy_consensus::OpReceiptEnvelope;

/// Describes the interface of the interop data provider.
#[async_trait]
pub trait InteropProvider {
    /// Fetch all receipts for a given block by hash.
    async fn block_receipts(&self, block_hash: B256) -> Vec<OpReceiptEnvelope>;
}
