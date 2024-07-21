//! Contains the [EthereumDataSource], which is a concrete implementation of the
//! [DataAvailabilityProvider] trait for the Ethereum protocol.

use crate::{
    sources::{BlobSource, CalldataSource, EthereumDataSourceVariant},
    traits::{BlobProvider, ChainProvider, DataAvailabilityProvider},
    types::{BlockInfo, RollupConfig},
};
use alloc::{boxed::Box, fmt::Debug};
use alloy_primitives::{Address, Bytes};
use anyhow::Result;
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
    /// The batch inbox address.
    pub batch_inbox_address: Address,
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
            signer: cfg
                .genesis
                .system_config
                .as_ref()
                .map(|sc| sc.batcher_address)
                .unwrap_or_default(),
            batch_inbox_address: cfg.batch_inbox_address,
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

    async fn open_data(&self, block_ref: &BlockInfo) -> Result<Self::DataIter> {
        let ecotone_enabled =
            self.ecotone_timestamp.map(|e| block_ref.timestamp >= e).unwrap_or(false);
        if ecotone_enabled {
            Ok(EthereumDataSourceVariant::Blob(BlobSource::new(
                self.chain_provider.clone(),
                self.blob_provider.clone(),
                self.batch_inbox_address,
                *block_ref,
                self.signer,
            )))
        } else {
            Ok(EthereumDataSourceVariant::Calldata(CalldataSource::new(
                self.chain_provider.clone(),
                self.batch_inbox_address,
                *block_ref,
                self.signer,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use alloy_consensus::TxEnvelope;
    use alloy_eips::eip2718::Decodable2718;
    use alloy_primitives::address;
    use kona_primitives::{BlockInfo, RollupConfig, SystemConfig};

    use crate::{
        sources::{EthereumDataSource, EthereumDataSourceVariant},
        traits::{
            test_utils::{TestBlobProvider, TestChainProvider},
            AsyncIterator, DataAvailabilityProvider,
        },
    };

    #[tokio::test]
    async fn test_validate_ethereum_data_source() {
        let chain = TestChainProvider::default();
        let blob = TestBlobProvider::default();
        let block_ref = BlockInfo::default();

        // If the ecotone_timestamp is not set, a Calldata source should be returned.
        let cfg = RollupConfig { ecotone_time: None, ..Default::default() };
        let data_source = EthereumDataSource::new(chain.clone(), blob.clone(), &cfg);
        let data_iter = data_source.open_data(&block_ref).await.unwrap();
        assert!(matches!(data_iter, EthereumDataSourceVariant::Calldata(_)));

        // If the ecotone_timestamp is set, and the block_ref timestamp is prior to the
        // ecotone_timestamp, a calldata source is created.
        let cfg = RollupConfig { ecotone_time: Some(100), ..Default::default() };
        let data_source = EthereumDataSource::new(chain, blob, &cfg);
        let data_iter = data_source.open_data(&block_ref).await.unwrap();
        assert!(matches!(data_iter, EthereumDataSourceVariant::Calldata(_)));

        // If the ecotone_timestamp is set, and the block_ref timestamp is greater than
        // or equal to the ecotone_timestamp, a Blob source is created.
        let block_ref = BlockInfo { timestamp: 101, ..Default::default() };
        let data_iter = data_source.open_data(&block_ref).await.unwrap();
        assert!(matches!(data_iter, EthereumDataSourceVariant::Blob(_)));
    }

    #[tokio::test]
    async fn test_open_ethereum_calldata_source_pre_ecotone() {
        let mut chain = TestChainProvider::default();
        let blob = TestBlobProvider::default();
        let batcher_address = address!("6887246668a3b87F54DeB3b94Ba47a6f63F32985");
        let batch_inbox = address!("FF00000000000000000000000000000000000010");
        let block_ref = BlockInfo { number: 10, ..Default::default() };

        let mut cfg = RollupConfig::default();
        cfg.genesis.system_config =
            Some(SystemConfig { batcher_addr: batcher_address, ..Default::default() });
        cfg.batch_inbox_address = batch_inbox;

        // load a test batcher transaction
        let raw_batcher_tx = include_bytes!("../../testdata/raw_batcher_tx.hex");
        let tx = TxEnvelope::decode_2718(&mut raw_batcher_tx.as_ref()).unwrap();
        chain.insert_block_with_transactions(10, block_ref, alloc::vec![tx]);

        let data_source = EthereumDataSource::new(chain, blob, &cfg);
        let mut data_iter = data_source.open_data(&block_ref).await.unwrap();
        assert!(matches!(data_iter, EthereumDataSourceVariant::Calldata(_)));

        // Should successfully retrieve a calldata batch from the block
        let calldata_batch = data_iter.next().await.unwrap().unwrap();
        assert_eq!(calldata_batch.len(), 119823);
    }
}
