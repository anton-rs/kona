//! Traits for the `kona-interop` crate.

use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::B256;
use async_trait::async_trait;
use op_alloy_consensus::OpReceiptEnvelope;

/// Describes the interface of the interop data provider. This provider is multiplexed over several chains, with each
/// method consuming a chain ID to determine the target chain.
#[async_trait]
pub trait InteropProvider {
    /// Fetch all receipts for a given block by hash.
    async fn block_receipts(&self, chain_id: u64, block_hash: B256) -> Vec<OpReceiptEnvelope>;

    /// Fetch the preimage of a message hash.
    async fn message_by_hash(&self, message_hash: B256) -> Option<Vec<u8>>;
}
