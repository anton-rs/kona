//! Contains a Factory for creating a calldata and blob provider.

use crate::{
    sources::{BlobSource, CalldataSource, EthereumDataSourceVariant},
    traits::{BlobProvider, ChainProvider, DataAvailabilityProvider},
    types::{BlockInfo, RollupConfig},
};
use alloc::{boxed::Box, fmt::Debug};
use alloy_primitives::{Address, Bytes};
use anyhow::{anyhow, Result};
use async_trait::async_trait;

/// A factory for creating an Ethereum data source provider.
#[derive(Debug, Clone, Copy)]
pub struct EthereumDataSource<C, B>
where
    C: ChainProvider + Send + Clone,
    B: BlobProvider + Clone,
{
    /// The chain provider to use for the factory.
    pub chain_provider: C,
    /// The blob provider
    pub blob_provider: B,
    /// The ecotone timestamp.
    pub ecotone_timestamp: Option<u64>,
    /// The L1 Signer.
    pub signer: Address,
}

impl<C, B> EthereumDataSource<C, B>
where
    C: ChainProvider + Send + Clone + Debug,
    B: BlobProvider + Clone + Debug,
{
    /// Creates a new factory.
    pub fn new(provider: C, blobs: B, cfg: &RollupConfig) -> Self {
        Self {
            chain_provider: provider,
            blob_provider: blobs,
            ecotone_timestamp: cfg.ecotone_time,
            signer: cfg.genesis.system_config.batcher_addr,
        }
    }
}

#[async_trait]
impl<C, B> DataAvailabilityProvider for EthereumDataSource<C, B>
where
    C: ChainProvider + Send + Sync + Clone + Debug,
    B: BlobProvider + Send + Sync + Clone + Debug,
{
    type Item = Bytes;
    type DataIter = EthereumDataSourceVariant<C, B>;

    async fn open_data(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter> {
        if let Some(ecotone) = self.ecotone_timestamp {
            let source = (block_ref.timestamp >= ecotone)
                .then(|| {
                    EthereumDataSourceVariant::Blob(BlobSource::new(
                        self.chain_provider.clone(),
                        self.blob_provider.clone(),
                        batcher_address,
                        *block_ref,
                        self.signer,
                    ))
                })
                .unwrap_or_else(|| {
                    EthereumDataSourceVariant::Calldata(CalldataSource::new(
                        self.chain_provider.clone(),
                        batcher_address,
                        *block_ref,
                        self.signer,
                    ))
                });
            Ok(source)
        } else {
            Err(anyhow!("No data source available"))
        }
    }
}
