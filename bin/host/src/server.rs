//! This module contains the [PreimageServer] struct and its implementation.

use anyhow::{anyhow, Result};
use kona_preimage::{
    errors::PreimageOracleError, HintReaderServer, PreimageOracleServer, PreimageServerBackend,
};
use std::sync::Arc;
use tokio::spawn;
use tracing::{error, info};

/// The [PreimageServer] is responsible for waiting for incoming preimage requests and
/// serving them to the client.
#[derive(Debug)]
pub struct PreimageServer<P, H, B> {
    /// The oracle server.
    oracle_server: P,
    /// The hint router.
    hint_reader: H,
    /// [PreimageServerBackend] that routes hints and retrieves preimages.
    backend: Arc<B>,
}

impl<P, H, B> PreimageServer<P, H, B>
where
    P: PreimageOracleServer + Send + Sync + 'static,
    H: HintReaderServer + Send + Sync + 'static,
    B: PreimageServerBackend + Send + Sync + 'static,
{
    /// Create a new [PreimageServer] with the given [PreimageOracleServer],
    /// [HintReaderServer], and [PreimageServerBackend].
    pub const fn new(oracle_server: P, hint_reader: H, backend: Arc<B>) -> Self {
        Self { oracle_server, hint_reader, backend }
    }

    /// Starts the [PreimageServer] and waits for incoming requests.
    pub async fn start(self) -> Result<()> {
        // Create the futures for the oracle server and hint router.
        let server = spawn(Self::start_oracle_server(self.oracle_server, self.backend.clone()));
        let hint_router = spawn(Self::start_hint_router(self.hint_reader, self.backend.clone()));

        // Race the two futures to completion, returning the result of the first one to finish.
        tokio::select! {
            s = server => s.map_err(|e| anyhow!(e))?,
            h = hint_router => h.map_err(|e| anyhow!(e))?,
        }
    }

    /// Starts the oracle server, which waits for incoming preimage requests and serves them to the
    /// client.
    async fn start_oracle_server(oracle_server: P, backend: Arc<B>) -> Result<()> {
        info!(target: "host-server", "Starting oracle server");
        loop {
            // Serve the next preimage request. This `await` will yield to the runtime
            // if no progress can be made.
            match oracle_server.next_preimage_request(backend.as_ref()).await {
                Ok(_) => continue,
                Err(PreimageOracleError::IOError(_)) => return Ok(()),
                Err(e) => {
                    error!("Failed to serve preimage request: {e}");
                    return Err(anyhow!("Failed to serve preimage request: {e}"));
                }
            }
        }
    }

    /// Starts the hint router, which waits for incoming hints and routes them to the appropriate
    /// handler.
    async fn start_hint_router(hint_reader: H, backend: Arc<B>) -> Result<()> {
        info!(target: "host-server", "Starting hint router");
        loop {
            // Route the next hint. This `await` will yield to the runtime if no progress can be
            // made.
            match hint_reader.next_hint(backend.as_ref()).await {
                Ok(_) => continue,
                Err(PreimageOracleError::IOError(_)) => return Ok(()),
                Err(e) => {
                    error!("Failed to serve route hint: {e}");
                    return Err(anyhow!("Failed to route hint: {e}"));
                }
            }
        }
    }
}
