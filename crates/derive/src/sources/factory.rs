//! Contains a Factory for creating a calldata and blob provider.

use crate::{
    sources::{BlobSource, CalldataSource, DataSource, PlasmaSource},
    traits::{BlobProvider, ChainProvider, DataAvailabilityProvider},
    types::{BlockID, BlockInfo, RollupConfig},
};
use alloc::{boxed::Box, fmt::Debug};
use alloy_primitives::{Address, Bytes};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_plasma::traits::{ChainProvider as PlasmaChainProvider, PlasmaInputFetcher};

/// A factory for creating a calldata and blob provider.
#[derive(Debug, Clone, Copy)]
pub struct DataSourceFactory<C, B, PCP, PIF, I>
where
    C: ChainProvider + Clone,
    B: BlobProvider + Clone,
    PCP: PlasmaChainProvider + Send + Clone,
    PIF: PlasmaInputFetcher<PCP> + Clone,
    I: Iterator<Item = Bytes> + Send + Clone,
{
    /// The chain provider to use for the factory.
    pub chain_provider: C,
    /// The plasma chain provider.
    pub plasma_chain_provider: PCP,
    /// The plasma iterator.
    pub plasma_source: I,
    /// The blob provider
    pub blob_provider: B,
    /// The plasma input fetcher.
    pub plasma_input_fetcher: PIF,
    /// The ecotone timestamp.
    pub ecotone_timestamp: Option<u64>,
    /// Whether or not plasma is enabled.
    pub plasma_enabled: bool,
    /// The L1 Signer.
    pub signer: Address,
}

impl<C, B, PCP, PIF, I> DataSourceFactory<C, B, PCP, PIF, I>
where
    C: ChainProvider + Clone + Debug,
    B: BlobProvider + Clone + Debug,
    PCP: PlasmaChainProvider + Send + Clone + Debug,
    PIF: PlasmaInputFetcher<PCP> + Clone + Debug,
    I: Iterator<Item = Bytes> + Send + Clone,
{
    /// Creates a new factory.
    pub fn new(provider: C, blobs: B, pcp: PCP, pif: PIF, s: I, cfg: &RollupConfig) -> Self {
        Self {
            chain_provider: provider,
            plasma_chain_provider: pcp,
            plasma_source: s,
            blob_provider: blobs,
            plasma_input_fetcher: pif,
            ecotone_timestamp: cfg.ecotone_time,
            plasma_enabled: cfg.is_plasma_enabled(),
            signer: cfg.genesis.system_config.batcher_addr,
        }
    }
}

#[async_trait]
impl<C, B, PCP, PIF, I> DataAvailabilityProvider for DataSourceFactory<C, B, PCP, PIF, I>
where
    C: ChainProvider + Send + Sync + Clone + Debug,
    B: BlobProvider + Send + Sync + Clone + Debug,
    PCP: PlasmaChainProvider + Send + Sync + Clone + Debug,
    PIF: PlasmaInputFetcher<PCP> + Send + Sync + Clone + Debug,
    I: Iterator<Item = Bytes> + Send + Sync + Clone + Debug,
{
    type Item = Bytes;
    type DataIter = DataSource<C, B, PCP, PIF, I>;

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
            let id = BlockID { hash: block_ref.hash, number: block_ref.number };
            Ok(DataSource::Plasma(PlasmaSource::new(
                self.plasma_chain_provider.clone(),
                self.plasma_input_fetcher.clone(),
                self.plasma_source.clone(),
                id,
            )))
        } else {
            Err(anyhow!("No data source available"))
        }
    }
}
