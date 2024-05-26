//! This module contains the [Fetcher] struct, which is responsible for fetching preimages from a
//! remote source.

use crate::{kv::KeyValueStore, util};
use alloy_primitives::{Bytes, B256};
use alloy_provider::{Provider, ReqwestProvider};
use anyhow::{anyhow, Result};
use kona_preimage::{PreimageKey, PreimageKeyType};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

mod hint;
pub use hint::HintType;

/// The [Fetcher] struct is responsible for fetching preimages from a remote source.
pub struct Fetcher<KV>
where
    KV: KeyValueStore,
{
    /// Key-value store for preimages.
    kv_store: Arc<RwLock<KV>>,
    /// L1 chain provider.
    l1_provider: ReqwestProvider,
    /// L2 chain provider.
    /// TODO: OP provider, N = Optimism
    #[allow(unused)]
    l2_provider: ReqwestProvider,
    /// The last hint that was received. [None] if no hint has been received yet.
    last_hint: Option<String>,
}

impl<KV> Fetcher<KV>
where
    KV: KeyValueStore,
{
    /// Create a new [Fetcher] with the given [KeyValueStore].
    pub fn new(
        kv_store: Arc<RwLock<KV>>,
        l1_provider: ReqwestProvider,
        l2_provider: ReqwestProvider,
    ) -> Self {
        Self { kv_store, l1_provider, l2_provider, last_hint: None }
    }

    /// Set the last hint to be received.
    pub fn hint(&mut self, hint: &str) {
        debug!(target: "fetcher", "Received hint: {hint}");
        self.last_hint = Some(hint.to_string());
    }

    /// Get the preimage for the given key.
    pub async fn get_preimage(&self, key: B256) -> Result<Vec<u8>> {
        debug!(target: "fetcher", "Pre-image requested. Key: {key}");

        // Acquire a read lock on the key-value store.
        let kv_lock = self.kv_store.read().await;
        let mut preimage = kv_lock.get(key).cloned();

        // Drop the read lock before beginning the loop.
        drop(kv_lock);

        // Use a loop to keep retrying the prefetch as long as the key is not found
        while preimage.is_none() && self.last_hint.is_some() {
            let hint = self.last_hint.as_ref().expect("Cannot be None");
            self.prefetch(hint).await?;

            let kv_lock = self.kv_store.read().await;
            preimage = kv_lock.get(key).cloned();
        }

        preimage.ok_or_else(|| anyhow!("Preimage not found."))
    }

    /// Fetch the preimage for the given hint and insert it into the key-value store.
    async fn prefetch(&self, hint: &str) -> Result<()> {
        let (hint_type, hint_data) = util::parse_hint(hint)?;
        debug!(target: "fetcher", "Fetching hint: {hint_type} {hint_data}");

        match hint_type {
            HintType::L1BlockHeader => {
                // Validate the hint data length.
                if hint_data.len() != 32 {
                    anyhow::bail!("Invalid hint data length: {}", hint_data.len());
                }

                // Fetch the raw header from the L1 chain provider.
                let hash: B256 = hint_data
                    .as_ref()
                    .try_into()
                    .map_err(|e| anyhow!("Failed to convert bytes to B256: {e}"))?;
                let raw_header: Bytes = self
                    .l1_provider
                    .client()
                    .request("debug_getRawHeader", [hash])
                    .await
                    .map_err(|e| anyhow!(e))?;

                // Acquire a lock on the key-value store and set the preimage.
                let mut kv_lock = self.kv_store.write().await;
                kv_lock.set(
                    PreimageKey::new(*hash, PreimageKeyType::Keccak256).into(),
                    raw_header.into(),
                );
            }
            HintType::L1Transactions => todo!(),
            HintType::L1Receipts => todo!(),
            HintType::L1Blob => todo!(),
            HintType::L1Precompile => todo!(),
            HintType::L2BlockHeader => todo!(),
            HintType::L2Transactions => todo!(),
            HintType::L2StateNode => todo!(),
            HintType::L2Code => todo!(),
            HintType::L2Output => todo!(),
        }

        Ok(())
    }
}
