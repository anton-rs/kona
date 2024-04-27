//! Contains a Factory for creating a calldata and blob provider.

use crate::{
    sources::{BlobSource, CalldataSource, DataSource, PlasmaSource},
    traits::{AsyncIterator, BlobProvider, DataAvailabilityProvider},
    types::{BlockID, BlockInfo, RollupConfig, StageResult},
};
use alloc::{boxed::Box, fmt::Debug};
use alloy_primitives::{Address, Bytes};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_plasma::traits::PlasmaInputFetcher;
use kona_providers::ChainProvider;

/// The [BaseDataSource] enum dispatches data source requests to the appropriate source.
#[derive(Debug, Clone)]
pub enum BaseDataSource<C: ChainProvider + Send + Clone, B: BlobProvider + Send + Clone> {
    /// A calldata source.
    Calldata(CalldataSource<C>),
    /// A blob source.
    Blob(BlobSource<C, B>),
}

#[async_trait]
impl<C, B> AsyncIterator for BaseDataSource<C, B>
where
    C: ChainProvider + Send + Clone,
    B: BlobProvider + Send + Clone,
{
    type Item = Bytes;

    async fn next(&mut self) -> Option<StageResult<Self::Item>> {
        match self {
            BaseDataSource::Calldata(c) => c.next().await,
            BaseDataSource::Blob(b) => b.next().await,
        }
    }
}

/// A factory for creating a calldata and blob provider.
#[derive(Debug, Clone, Copy)]
pub struct DataSourceFactory<C, B, PIF>
where
    C: ChainProvider + Send + Clone,
    B: BlobProvider + Clone,
    PIF: PlasmaInputFetcher<C> + Clone,
{
    /// The chain provider to use for the factory.
    pub chain_provider: C,
    /// The blob provider
    pub blob_provider: Option<B>,
    /// The plasma input fetcher.
    pub plasma_input_fetcher: Option<PIF>,
    /// The ecotone timestamp.
    pub ecotone_timestamp: Option<u64>,
    /// Whether or not plasma is enabled.
    pub plasma_enabled: bool,
    /// The L1 Signer.
    pub signer: Address,
}

impl<C, B, PIF> DataSourceFactory<C, B, PIF>
where
    C: ChainProvider + Send + Clone + Debug,
    B: BlobProvider + Clone + Debug,
    PIF: PlasmaInputFetcher<C> + Clone + Debug,
{
    /// Creates a new factory.
    pub fn new(provider: C, blobs: Option<B>, pif: Option<PIF>, cfg: &RollupConfig) -> Self {
        Self {
            chain_provider: provider,
            blob_provider: blobs,
            plasma_input_fetcher: pif,
            ecotone_timestamp: cfg.ecotone_time,
            plasma_enabled: cfg.is_plasma_enabled(),
            signer: cfg.genesis.system_config.batcher_addr,
        }
    }
}

#[async_trait]
impl<C, B, PIF> DataAvailabilityProvider for DataSourceFactory<C, B, PIF>
where
    C: ChainProvider + Send + Sync + Clone + Debug,
    B: BlobProvider + Send + Sync + Clone + Debug,
    PIF: PlasmaInputFetcher<C> + Send + Sync + Clone + Debug,
{
    type Item = Bytes;
    type DataIter = DataSource<C, B, PIF>;

    async fn open_data(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter> {
        let ecotone = self.ecotone_timestamp.map(|t| block_ref.timestamp >= t).unwrap_or(false);
        let source = if ecotone {
            match self.blob_provider {
                Some(_) => BaseDataSource::Blob(BlobSource::new(
                    self.chain_provider.clone(),
                    self.blob_provider.clone().expect("blob provider must be set"),
                    batcher_address,
                    *block_ref,
                    self.signer,
                )),
                None => {
                    return Err(anyhow!("No blob provider available"));
                }
            }
        } else {
            BaseDataSource::Calldata(CalldataSource::new(
                self.chain_provider.clone(),
                batcher_address,
                *block_ref,
                self.signer,
            ))
        };
        if self.plasma_enabled {
            let pif = match &self.plasma_input_fetcher {
                Some(p) => p,
                None => {
                    return Err(anyhow!("No plasma input fetcher available"));
                }
            };
            let id = BlockID { hash: block_ref.hash, number: block_ref.number };
            return Ok(DataSource::Plasma(PlasmaSource::new(
                self.chain_provider.clone(),
                pif.clone(),
                source,
                id,
            )));
        }
        match source {
            BaseDataSource::Blob(b) => Ok(DataSource::Blob(b)),
            BaseDataSource::Calldata(c) => Ok(DataSource::Calldata(c)),
        }
    }
}
