//! Contains a Factory for creating a calldata and blob provider.

use crate::sources::{BlobSource, CalldataSource, DataSource, PlasmaSource};
use crate::traits::{BlobProvider, ChainProvider, DataAvailabilityProvider};
use crate::types::{BlockInfo, RollupConfig, StageResult};
use alloc::boxed::Box;
use alloc::fmt::Debug;
use alloy_primitives::Address;
use anyhow::{anyhow, Result};
use async_trait::async_trait;

/// A factory for creating a calldata and blob provider.
#[derive(Debug, Clone, Copy)]
pub struct DataSourceFactory<C, B>
where
    C: ChainProvider + Clone,
    B: BlobProvider + Clone,
{
    /// The chain provider to use for the factory.
    pub chain_provider: C,
    /// The blob provider
    pub blob_provider: B,
    /// The ecotone timestamp.
    pub ecotone_timestamp: Option<u64>,
    /// Whether or not plasma is enabled.
    pub plasma_enabled: bool,
    /// The L1 Signer.
    pub signer: Address,
}

impl<C, B> DataSourceFactory<C, B>
where
    C: ChainProvider + Clone + Debug,
    B: BlobProvider + Clone + Debug,
{
    /// Creates a new factory.
    pub fn new(provider: C, blobs: B, cfg: RollupConfig) -> Self {
        Self {
            chain_provider: provider,
            blob_provider: blobs,
            ecotone_timestamp: cfg.ecotone_time,
            plasma_enabled: cfg.is_plasma_enabled(),
            signer: cfg.l1_signer_address(),
        }
    }
}

#[async_trait]
impl<C, B> DataAvailabilityProvider for DataSourceFactory<C, B>
where
    C: ChainProvider + Send + Sync + Clone + Debug,
    B: BlobProvider + Send + Sync + Clone + Debug,
{
    type Item = StageResult<alloy_primitives::Bytes>;
    type DataIter = DataSource<C, B>;

    async fn open_data(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter> {
        if let Some(ecotone) = self.ecotone_timestamp {
            let source = (block_ref.timestamp >= ecotone)
                .then(|| {
                    DataSource::Blob(BlobSource::new(
                        self.chain_provider.clone(),
                        self.blob_provider.clone(),
                        batcher_address,
                        *block_ref,
                        self.signer,
                    ))
                })
                .unwrap_or_else(|| {
                    DataSource::Calldata(CalldataSource::new(
                        self.chain_provider.clone(),
                        batcher_address,
                        *block_ref,
                        self.signer,
                    ))
                });
            Ok(source)
        } else if self.plasma_enabled {
            Ok(DataSource::Plasma(PlasmaSource::new(
                self.chain_provider.clone(),
            )))
        } else {
            Err(anyhow!("No data source available"))
        }
    }
}
