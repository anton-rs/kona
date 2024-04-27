//! Contains an online implementation of the [DataAvailabilityProvider] trait.

use async_trait::async_trait;
use alloy_providers::ReqwestProvider;
use crate::traits::DataAvailabilityProvider;

/// A data availability provider that fetches data from an Alloy chain provider.
#[derive(Debug)]
pub struct AlloyDataAvailabilityProvider {
    /// The inner [reqwest::Client] provider.
    inner: ReqwestProvider,
}

impl AlloyDataAvailabilityProvider {
    /// Creates a new instance of the [AlloyDataAvailabilityProvider].
    pub fn new(inner: ReqwestProvider) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl DataAvailabilityProvider for AlloyDataAvailabilityProvider {
    type Item = Bytes;
    type DataIter = impl AsyncIterator<Item = Self::Item> + Send + Debug;

    async fn open_data(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter> {
        let block_hash = block_ref.hash;
        let batcher_address = batcher_address.to_string();
        let url = format!(
            "{}/eth/v1/data/availability/{}/{}",
            self.inner.url(),
            block_hash,
            batcher_address
        );

        let response = self.inner.client().get(&url).send().await?;
        let data = response.bytes().await?;

        Ok(data.into_iter())
    }
}
