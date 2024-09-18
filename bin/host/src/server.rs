//! This module contains the [PreimageServer] struct and its implementation.

use crate::{
    fetcher::Fetcher,
    kv::KeyValueStore,
    preimage::{
        OfflineHintRouter, OfflinePreimageFetcher, OnlineHintRouter, OnlinePreimageFetcher,
    },
};
use anyhow::{anyhow, Result};
use kona_preimage::{
    HintReaderServer, HintRouter, PreimageFetcher, PreimageOracleServer, PreimageServerError,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;

/// The [PreimageServer] is responsible for waiting for incoming preimage requests and
/// serving them to the client.
#[derive(Debug)]
pub struct PreimageServer<P, H, KV>
where
    P: PreimageOracleServer,
    H: HintReaderServer,
    KV: KeyValueStore + ?Sized,
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
    KV: KeyValueStore + Send + Sync + ?Sized + 'static,
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
        let server_fut = Self::start_oracle_server(
            self.kv_store.clone(),
            self.fetcher.clone(),
            self.oracle_server,
        );
        let hinter_fut = Self::start_hint_router(self.hint_reader, self.fetcher);

        // Spawn tasks for the futures and wait for them to complete.
        let server = tokio::task::spawn(server_fut);
        let hint_router = tokio::task::spawn(hinter_fut);
        tokio::select! {
            s = server => s.map_err(|e| anyhow!(e))?,
            h = hint_router => h.map_err(|e| anyhow!(e))?,
        }
    }

    /// Starts the oracle server, which waits for incoming preimage requests and serves them to the
    /// client.
    async fn start_oracle_server(
        kv_store: Arc<RwLock<KV>>,
        fetcher: Option<Arc<RwLock<Fetcher<KV>>>>,
        oracle_server: P,
    ) -> Result<()> {
        #[inline(always)]
        async fn do_loop<F, P>(fetcher: &F, server: &P) -> Result<()>
        where
            F: PreimageFetcher + Send + Sync,
            P: PreimageOracleServer,
        {
            loop {
                match server.next_preimage_request(fetcher).await {
                    Ok(_) => (),
                    Err(PreimageServerError::BrokenPipe(_)) => return Ok(()),
                    Err(PreimageServerError::Other(e)) => {
                        error!("Failed to serve preimage request: {e:?}");
                        return Err(e);
                    }
                }
            }
        }

        if let Some(fetcher) = fetcher.as_ref() {
            do_loop(&OnlinePreimageFetcher::new(Arc::clone(fetcher)), &oracle_server).await
        } else {
            do_loop(&OfflinePreimageFetcher::new(Arc::clone(&kv_store)), &oracle_server).await
        }
    }

    /// Starts the hint router, which waits for incoming hints and routes them to the appropriate
    /// handler.
    async fn start_hint_router(
        hint_reader: H,
        fetcher: Option<Arc<RwLock<Fetcher<KV>>>>,
    ) -> Result<()> {
        #[inline(always)]
        async fn do_loop<R, H>(router: &R, server: &H) -> Result<()>
        where
            R: HintRouter + Send + Sync,
            H: HintReaderServer,
        {
            loop {
                match server.next_hint(router).await {
                    Ok(_) => (),
                    Err(PreimageServerError::BrokenPipe(_)) => return Ok(()),
                    Err(PreimageServerError::Other(e)) => {
                        error!("Failed to serve preimage request: {e:?}");
                        return Err(e);
                    }
                }
            }
        }

        if let Some(fetcher) = fetcher {
            do_loop(&OnlineHintRouter::new(Arc::clone(&fetcher)), &hint_reader).await
        } else {
            do_loop(&OfflineHintRouter, &hint_reader).await
        }
    }
}
