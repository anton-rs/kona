//! Contains the [HostOrchestrator] trait, which defines entry points for the host to run a given
//! module.

use crate::{Fetcher, PreimageServer, SharedKeyValueStore};
use anyhow::Result;
use async_trait::async_trait;
use kona_preimage::{
    BidirectionalChannel, HintReader, HintWriter, NativeChannel, OracleReader, OracleServer,
};
use kona_std_fpvm::{FileChannel, FileDescriptor};
use std::sync::Arc;
use tokio::{sync::RwLock, task};

/// The host<->client communication channels. The client channels are optional, as the client may
/// not be running in the same process as the host.
#[derive(Debug)]
struct HostComms {
    /// The host<->client hint channel.
    pub hint: BidirectionalChannel,
    /// The host<->client preimage channel.
    pub preimage: BidirectionalChannel,
}

/// The host->client communication channels when running in detached mode. The client channels are
/// held in a separate process.
#[derive(Debug)]
struct DetachedHostComms {
    /// The host->client hint channel.
    pub hint: FileChannel,
    /// The host->client preimage channel.
    pub preimage: FileChannel,
}

/// The orchestrator is responsible for starting the host and client program, and managing the
/// communication between them. It is the entry point for the host to run a given module.
///
/// This trait is specific to running both the host and client program in-process. For detached
/// mode, see [DetachedHostOrchestrator].
#[async_trait]
pub trait HostOrchestrator {
    /// A collection of the providers that the host can use to reference remote resources.
    type Providers;

    /// Instantiates the providers for the host's fetcher.
    async fn create_providers(&self) -> Result<Option<Self::Providers>>;

    /// Constructs the [KeyValueStore] for the host.
    ///
    /// [KeyValueStore]: crate::KeyValueStore
    fn create_key_value_store(&self) -> Result<SharedKeyValueStore>;

    /// Creates a [Fetcher] for the host program's preimage server.
    fn create_fetcher(
        &self,
        providers: Option<Self::Providers>,
        kv_store: SharedKeyValueStore,
    ) -> Option<Arc<RwLock<impl Fetcher + Send + Sync + 'static>>>;

    /// Runs the client program natively and returns the exit code.
    async fn run_client_native(
        hint_reader: HintWriter<NativeChannel>,
        oracle_reader: OracleReader<NativeChannel>,
    ) -> Result<()>;

    /// Starts the host and client program in-process.
    async fn start(&self) -> Result<()> {
        let comms = HostComms {
            hint: BidirectionalChannel::new()?,
            preimage: BidirectionalChannel::new()?,
        };
        let kv_store = self.create_key_value_store()?;
        let providers = self.create_providers().await?;
        let fetcher = self.create_fetcher(providers, kv_store.clone());

        let server_task = task::spawn(
            PreimageServer::new(
                OracleServer::new(comms.preimage.host),
                HintReader::new(comms.hint.host),
                kv_store,
                fetcher,
            )
            .start(),
        );
        let client_task = task::spawn(Self::run_client_native(
            HintWriter::new(comms.hint.client),
            OracleReader::new(comms.preimage.client),
        ));

        let (_, client_result) = tokio::try_join!(server_task, client_task)?;

        // Bubble up the exit status of the client program.
        std::process::exit(client_result.is_err() as i32);
    }
}

/// The orchestrator for starting the host in detached mode, with the client program running in a
/// separate process.
#[async_trait]
pub trait DetachedHostOrchestrator: HostOrchestrator {
    /// Returns whether the host is running in detached mode.
    fn is_detached(&self) -> bool;

    /// Starts the host in detached mode, with the client program running in a separate process.
    async fn run_detached(&self) -> Result<()> {
        let comms = DetachedHostComms {
            hint: FileChannel::new(FileDescriptor::HintRead, FileDescriptor::HintWrite),
            preimage: FileChannel::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite),
        };
        let kv_store = self.create_key_value_store()?;
        let providers = self.create_providers().await?;
        let fetcher = self.create_fetcher(providers, kv_store.clone());

        PreimageServer::new(
            OracleServer::new(comms.preimage),
            HintReader::new(comms.hint),
            kv_store,
            fetcher,
        )
        .start()
        .await
    }

    /// Override for [HostOrchestrator::start] that starts the host in detached mode,
    /// if [DetachedHostOrchestrator::is_detached] returns `true`.
    async fn run(&self) -> Result<()> {
        if self.is_detached() {
            self.run_detached().await
        } else {
            HostOrchestrator::start(self).await
        }
    }
}
