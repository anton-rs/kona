//! This module contains the [PreimageServer] struct and its implementation.

use crate::{fetcher::Fetcher, kv::KeyValueStore};
use anyhow::{anyhow, Result};
use kona_preimage::{HintReaderServer, PreimageKey, PreimageOracleServer};
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::RwLock;
use tracing::debug;

/// The [PreimageServer] is responsible for waiting for incoming preimage requests and
/// serving them to the client.
pub struct PreimageServer<P, H, KV>
where
    P: PreimageOracleServer,
    H: HintReaderServer,
    KV: KeyValueStore,
{
    /// The oracle server.
    oracle_server: P,
    /// The hint router.
    hint_reader: H,
    /// Key-value store for preimages.
    kv_store: Arc<RwLock<KV>>,
    /// The fetcher for fetching preimages from a remote source. If [None], the server will only
    /// serve preimages that are already in the key-value store.
    fetcher: Option<Arc<RwLock<Fetcher<KV>>>>,
}

impl<P, H, KV> PreimageServer<P, H, KV>
where
    P: PreimageOracleServer + Send + Sync + 'static,
    H: HintReaderServer + Send + Sync + 'static,
    KV: KeyValueStore + Send + Sync + 'static,
{
    /// Create a new [PreimageServer] with the given [PreimageOracleServer],
    /// [HintReaderServer], and [KeyValueStore]. Holds onto the file descriptors for the pipes
    /// that are created, so that the pipes are not closed until the server is dropped.
    pub fn new(
        oracle_server: P,
        hint_reader: H,
        kv_store: Arc<RwLock<KV>>,
        fetcher: Option<Arc<RwLock<Fetcher<KV>>>>,
    ) -> Self {
        Self { oracle_server, hint_reader, kv_store, fetcher }
    }

    /// Starts the [PreimageServer] and waits for incoming requests.
    pub async fn start(self) -> Result<()> {
        // Create the futures for the oracle server and hint router.
        let server_fut =
            Self::start_oracle_server(self.kv_store, self.fetcher.clone(), self.oracle_server);
        let hinter_fut = Self::start_hint_router(self.hint_reader, self.fetcher);

        // Spawn tasks for the futures and wait for them to complete.
        let server = tokio::task::spawn(server_fut);
        let hint_router = tokio::task::spawn(hinter_fut);
        tokio::try_join!(server, hint_router).map_err(|e| anyhow!(e))?;

        Ok(())
    }

    /// Starts the oracle server, which waits for incoming preimage requests and serves them to the
    /// client.
    async fn start_oracle_server(
        kv_store: Arc<RwLock<KV>>,
        fetcher: Option<Arc<RwLock<Fetcher<KV>>>>,
        oracle_server: P,
    ) {
        #[allow(clippy::type_complexity)]
        let get_preimage =
            |key: PreimageKey| -> Pin<Box<dyn Future<Output = Result<Vec<u8>>> + Send>> {
                if let Some(fetcher) = fetcher.as_ref() {
                    // If a fetcher is present, use it to fetch the preimage.
                    Box::pin(async move {
                        let fetcher = fetcher.read().await;
                        fetcher.get_preimage(key.into()).await
                    })
                } else {
                    // Otherwise, use the key-value store to fetch the preimage when in offline
                    // mode.
                    let kv_store = kv_store.as_ref();
                    Box::pin(async move {
                        kv_store
                            .read()
                            .await
                            .get(key.into())
                            .ok_or_else(|| anyhow!("Preimage not found"))
                            .cloned()
                    })
                }
            };

        loop {
            // TODO: More granular error handling. Some errors here are expected, such as the client
            // closing the pipe, while others are not and should throw.
            if oracle_server.next_preimage_request(get_preimage).await.is_err() {
                break;
            }
        }
    }

    /// Starts the hint router, which waits for incoming hints and routes them to the appropriate
    /// handler.
    async fn start_hint_router(hint_reader: H, fetcher: Option<Arc<RwLock<Fetcher<KV>>>>) {
        let route_hint = |hint: String| -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
            if let Some(fetcher) = fetcher.as_ref() {
                let fetcher = Arc::clone(fetcher);
                Box::pin(async move {
                    fetcher.write().await.hint(&hint);
                    Ok(())
                })
            } else {
                Box::pin(async move {
                    debug!(target: "preimage_server", "Received hint in offline mode: {}", &hint);
                    Ok(())
                })
            }
        };

        loop {
            // TODO: More granular error handling. Some errors here are expected, such as the client
            // closing the pipe, while others are not and should throw.
            if hint_reader.next_hint(route_hint).await.is_err() {
                break;
            }
        }
    }
}
