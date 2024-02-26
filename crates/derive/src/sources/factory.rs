//! Contains a Factory for creating a calldata and blob provider.

use crate::sources::{BlobSource, CalldataSource, PlasmaSource};
use crate::traits::{ChainProvider, DataAvailabilityProvider};
use crate::types::{BlockInfo, RollupConfig};
use alloy_primitives::{Address, Bytes};
use anyhow::{anyhow, Result};

/// A factory for creating a calldata and blob provider.
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
}

impl<F: ChainProvider + Clone> DataSourceFactory<F> {
    /// Creates a new factory.
    pub fn new(provider: F, cfg: RollupConfig) -> Self {
        Self {
            chain_provider: Box::new(provider),
            ecotone_timestamp: cfg.ecotone_time,
            plasma_enabled: cfg.is_plasma_enabled(),
        }
    }
}

impl<F: ChainProvider + Clone> DataAvailabilityProvider for DataSourceFactory<F> {
    type DataIter<T> = F::DataIter<T>;

    async fn open_data<T: Into<Bytes>>(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter<T>> {
        if let Some(ecotone) = self.ecotone_timestamp {
            if block_ref.timestamp >= ecotone {
                return Ok(BlobSource::new());
            }
            return Ok(CalldataSource::new(
                self.chain_provider.clone(),
                batcher_address,
                *block_ref,
            ));
        }
        if self.plasma_enabled {
            return Ok(PlasmaSource::new());
        }
        return Err(anyhow!("No data source available"));
    }
}
