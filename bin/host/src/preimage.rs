//! Contains the implementations of the [HintRouter] and [PreimageFetcher] traits.]

use crate::{fetcher::Fetcher, kv::KeyValueStore};
use async_trait::async_trait;
use kona_preimage::{
    errors::{PreimageOracleError, PreimageOracleResult},
    HintRouter, PreimageFetcher, PreimageKey,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// A [Fetcher]-backed implementation of the [PreimageFetcher] trait.
#[derive(Debug)]
pub struct OnlinePreimageFetcher<KV>
where
    KV: KeyValueStore + ?Sized,
{
    inner: Arc<RwLock<Fetcher<KV>>>,
}

#[async_trait]
impl<KV> PreimageFetcher for OnlinePreimageFetcher<KV>
where
    KV: KeyValueStore + Send + Sync + ?Sized,
{
    async fn get_preimage(&self, key: PreimageKey) -> PreimageOracleResult<Vec<u8>> {
        let fetcher = self.inner.read().await;
        fetcher
            .get_preimage(key.into())
            .await
            .map_err(|e| PreimageOracleError::Other(e.to_string()))
    }
}

impl<KV> OnlinePreimageFetcher<KV>
where
    KV: KeyValueStore + ?Sized,
{
    /// Create a new [OnlinePreimageFetcher] from the given [Fetcher].
    pub const fn new(fetcher: Arc<RwLock<Fetcher<KV>>>) -> Self {
        Self { inner: fetcher }
    }
}

/// A [KeyValueStore]-backed implementation of the [PreimageFetcher] trait.
#[derive(Debug)]
pub struct OfflinePreimageFetcher<KV>
where
    KV: KeyValueStore + ?Sized,
{
    inner: Arc<RwLock<KV>>,
}

#[async_trait]
impl<KV> PreimageFetcher for OfflinePreimageFetcher<KV>
where
    KV: KeyValueStore + Send + Sync + ?Sized,
{
    async fn get_preimage(&self, key: PreimageKey) -> PreimageOracleResult<Vec<u8>> {
        let kv_store = self.inner.read().await;
        kv_store.get(key.into()).ok_or(PreimageOracleError::KeyNotFound)
    }
}

impl<KV> OfflinePreimageFetcher<KV>
where
    KV: KeyValueStore + ?Sized,
{
    /// Create a new [OfflinePreimageFetcher] from the given [KeyValueStore].
    pub const fn new(kv_store: Arc<RwLock<KV>>) -> Self {
        Self { inner: kv_store }
    }
}

/// A [Fetcher]-backed implementation of the [HintRouter] trait.
#[derive(Debug)]
pub struct OnlineHintRouter<KV>
where
    KV: KeyValueStore + ?Sized,
{
    inner: Arc<RwLock<Fetcher<KV>>>,
}

#[async_trait]
impl<KV> HintRouter for OnlineHintRouter<KV>
where
    KV: KeyValueStore + Send + Sync + ?Sized,
{
    async fn route_hint(&self, hint: String) -> PreimageOracleResult<()> {
        let mut fetcher = self.inner.write().await;
        fetcher.hint(&hint);
        Ok(())
    }
}

impl<KV> OnlineHintRouter<KV>
where
    KV: KeyValueStore + ?Sized,
{
    /// Create a new [OnlineHintRouter] from the given [Fetcher].
    pub const fn new(fetcher: Arc<RwLock<Fetcher<KV>>>) -> Self {
        Self { inner: fetcher }
    }
}

/// An [OfflineHintRouter] is a [HintRouter] that does nothing.
#[derive(Debug)]
pub struct OfflineHintRouter;

#[async_trait]
impl HintRouter for OfflineHintRouter {
    async fn route_hint(&self, _hint: String) -> PreimageOracleResult<()> {
        Ok(())
    }
}
