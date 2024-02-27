//! Contains a Factory for creating a calldata and blob provider.

use crate::sources::{BlobSource, CalldataSource, DataSource, PlasmaSource};
use crate::traits::{ChainProvider, DataAvailabilityProvider};
use crate::types::{BlockInfo, RollupConfig};
use alloc::boxed::Box;
use alloc::fmt::Debug;
use alloy_primitives::Address;
use anyhow::{anyhow, Result};
use async_trait::async_trait;

/// A factory for creating a calldata and blob provider.
#[derive(Debug, Clone, Copy)]
pub struct DataSourceFactory<CP>
where
    CP: ChainProvider + Clone,
{
    /// The chain provider to use for the factory.
    pub chain_provider: CP,
    /// The ecotone timestamp.
    pub ecotone_timestamp: Option<u64>,
    /// Whether or not plasma is enabled.
    pub plasma_enabled: bool,
    /// The L1 Signer.
    pub signer: Address,
}

impl<F: ChainProvider + Clone> DataSourceFactory<F> {
    /// Creates a new factory.
    pub fn new(provider: F, cfg: RollupConfig) -> Self {
        Self {
            chain_provider: provider,
            ecotone_timestamp: cfg.ecotone_time,
            plasma_enabled: cfg.is_plasma_enabled(),
            signer: cfg.l1_signer_address(),
        }
    }
}

#[async_trait]
impl<F: ChainProvider + Send + Sync + Clone + Debug> DataAvailabilityProvider
    for DataSourceFactory<F>
{
    type DataIter = DataSource<F>;

    async fn open_data(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter> {
        if let Some(ecotone) = self.ecotone_timestamp {
            if block_ref.timestamp >= ecotone {
                return Ok(DataSource::Blob(BlobSource::new()));
            }
            return Ok(DataSource::Calldata(CalldataSource::new(
                self.chain_provider.clone(),
                batcher_address,
                *block_ref,
                self.signer,
            )));
        }
        if self.plasma_enabled {
            return Ok(DataSource::Plasma(PlasmaSource::new()));
        }
        return Err(anyhow!("No data source available"));
    }
}
