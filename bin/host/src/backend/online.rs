//! Contains the [OnlineHostBackend] definition.

use crate::SharedKeyValueStore;
use anyhow::Result;
use async_trait::async_trait;
use kona_preimage::{
    errors::{PreimageOracleError, PreimageOracleResult},
    HintRouter, PreimageFetcher, PreimageKey,
};
use std::{hash::Hash, str::FromStr, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, trace, warn};

/// The [OnlineHostBackendCfg] trait is used to define the type configuration for the
/// [OnlineHostBackend].
pub trait OnlineHostBackendCfg {
    /// The hint type describing the range of hints that can be received.
    type Hint: FromStr + Hash + Eq + PartialEq + Send + Sync;

    /// The providers that are used to fetch data in response to hints.
    type Providers: Send + Sync;
}

/// A [HintHandler] is an interface for receiving hints, fetching remote data, and storing it in the
/// key-value store.
#[async_trait]
pub trait HintHandler {
    /// The type configuration for the [HintHandler].
    type Cfg: OnlineHostBackendCfg;

    /// Fetches data in response to a hint.
    async fn fetch_hint(
        hint: <Self::Cfg as OnlineHostBackendCfg>::Hint,
        cfg: &Self::Cfg,
        providers: &<Self::Cfg as OnlineHostBackendCfg>::Providers,
        kv: SharedKeyValueStore,
    ) -> Result<()>;
}

/// The [OnlineHostBackend] is a [HintRouter] and [PreimageFetcher] that is used to fetch data from
/// remote sources in response to hints.
///
/// [PreimageKey]: kona_preimage::PreimageKey
#[allow(missing_debug_implementations)]
pub struct OnlineHostBackend<C, H>
where
    C: OnlineHostBackendCfg,
    H: HintHandler,
{
    /// The configuration that is used to route hints.
    cfg: C,
    /// The key-value store that is used to store preimages.
    kv: SharedKeyValueStore,
    /// The providers that are used to fetch data in response to hints.
    providers: C::Providers,
    /// The last hint that was received.
    last_hint: Arc<RwLock<Option<String>>>,
    /// Phantom marker for the [HintHandler].
    _hint_handler: std::marker::PhantomData<H>,
}

impl<C, H> OnlineHostBackend<C, H>
where
    C: OnlineHostBackendCfg,
    H: HintHandler,
{
    /// Creates a new [HintHandler] with the given configuration, key-value store, providers, and
    /// external configuration.
    pub fn new(cfg: C, kv: SharedKeyValueStore, providers: C::Providers, _: H) -> Self {
        Self {
            cfg,
            kv,
            providers,
            last_hint: Arc::new(RwLock::new(None)),
            _hint_handler: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<C, H> HintRouter for OnlineHostBackend<C, H>
where
    C: OnlineHostBackendCfg + Send + Sync,
    H: HintHandler<Cfg = C> + Send + Sync,
{
    /// Set the last hint to be received.
    async fn route_hint(&self, hint: String) -> PreimageOracleResult<()> {
        trace!(target: "host-backend", "Received hint: {hint}");
        let mut hint_lock = self.last_hint.write().await;
        hint_lock.replace(hint);
        Ok(())
    }
}

#[async_trait]
impl<C, H> PreimageFetcher for OnlineHostBackend<C, H>
where
    C: OnlineHostBackendCfg + Send + Sync,
    H: HintHandler<Cfg = C> + Send + Sync,
{
    /// Get the preimage for the given key.
    async fn get_preimage(&self, key: PreimageKey) -> PreimageOracleResult<Vec<u8>> {
        trace!(target: "host-backend", "Pre-image requested. Key: {key}");

        // Acquire a read lock on the key-value store.
        let kv_lock = self.kv.read().await;
        let mut preimage = kv_lock.get(key.into());

        // Drop the read lock before beginning the retry loop.
        drop(kv_lock);

        // Use a loop to keep retrying the prefetch as long as the key is not found
        while preimage.is_none() {
            if let Some(hint) = self.last_hint.read().await.as_ref() {
                let parsed_hint =
                    hint.parse::<C::Hint>().map_err(|_| PreimageOracleError::KeyNotFound)?;
                let value =
                    H::fetch_hint(parsed_hint, &self.cfg, &self.providers, self.kv.clone()).await;

                if let Err(e) = value {
                    error!(target: "host-backend", "Failed to prefetch hint: {e}");
                    warn!(target: "host-backend", "Retrying hint fetch: {hint}");
                    continue;
                }

                let kv_lock = self.kv.read().await;
                preimage = kv_lock.get(key.into());
            }
        }

        preimage.ok_or(PreimageOracleError::KeyNotFound)
    }
}
