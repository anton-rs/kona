//! [SingleChainHostCli]'s [HostOrchestrator] + [DetachedHostOrchestrator] implementations.

use super::{LocalKeyValueStore, SingleChainFetcher, SingleChainHostCli};
use crate::{
    eth::{http_provider, OnlineBlobProvider},
    orchestrator::{DetachedHostOrchestrator, HostOrchestrator},
};
use alloy_provider::ReqwestProvider;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_preimage::{HintWriter, NativeChannel, OracleReader};
use kona_preimage_server::{
    DiskKeyValueStore, Fetcher, MemoryKeyValueStore, SharedKeyValueStore, SplitKeyValueStore,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// The providers required for the single chain host.
#[derive(Debug)]
pub struct SingleChainProviders {
    /// The L1 EL provider.
    l1_provider: ReqwestProvider,
    /// The L1 beacon node provider.
    blob_provider: OnlineBlobProvider,
    /// The L2 EL provider.
    l2_provider: ReqwestProvider,
}

#[async_trait]
impl HostOrchestrator for SingleChainHostCli {
    type Providers = SingleChainProviders;

    async fn create_providers(&self) -> Result<Self::Providers> {
        let blob_provider = OnlineBlobProvider::new_http(
            self.l1_beacon_address.clone().ok_or(anyhow!("Beacon API URL must be set"))?,
        )
        .await
        .map_err(|e| anyhow!("Failed to load blob provider configuration: {e}"))?;
        let l1_provider =
            http_provider(self.l1_node_address.as_ref().ok_or(anyhow!("Provider must be set"))?);
        let l2_provider = http_provider(
            self.l2_node_address.as_ref().ok_or(anyhow!("L2 node address must be set"))?,
        );

        Ok(SingleChainProviders { l1_provider, blob_provider, l2_provider })
    }

    fn create_fetcher(
        &self,
        providers: Self::Providers,
        kv_store: SharedKeyValueStore,
    ) -> Option<Arc<RwLock<impl Fetcher + Send + Sync + 'static>>> {
        (!self.is_offline()).then(|| {
            Arc::new(RwLock::new(SingleChainFetcher::new(
                kv_store,
                providers.l1_provider,
                providers.blob_provider,
                providers.l2_provider,
                self.agreed_l2_head_hash,
            )))
        })
    }

    fn create_key_value_store(&self) -> Result<SharedKeyValueStore> {
        let local_kv_store = LocalKeyValueStore::new(self.clone());

        let kv_store: SharedKeyValueStore = if let Some(ref data_dir) = self.data_dir {
            let disk_kv_store = DiskKeyValueStore::new(data_dir.clone());
            let split_kv_store = SplitKeyValueStore::new(local_kv_store, disk_kv_store);
            Arc::new(RwLock::new(split_kv_store))
        } else {
            let mem_kv_store = MemoryKeyValueStore::new();
            let split_kv_store = SplitKeyValueStore::new(local_kv_store, mem_kv_store);
            Arc::new(RwLock::new(split_kv_store))
        };

        Ok(kv_store)
    }

    async fn run_client_native(
        hint_reader: HintWriter<NativeChannel>,
        oracle_reader: OracleReader<NativeChannel>,
    ) -> Result<()> {
        kona_client::single::run(oracle_reader, hint_reader, None).await.map_err(Into::into)
    }
}

#[async_trait]
impl DetachedHostOrchestrator for SingleChainHostCli {
    fn is_detached(&self) -> bool {
        self.server
    }
}
