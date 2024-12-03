/*
use crate::alloc::string::ToString;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use kona_preimage::CommsClient;

use crate::errors::OracleProviderError;

use super::OracleEigenDAProvider;

#[derive(Debug, Clone)]
pub struct OracleAltDAProvider<T: CommsClient> {
    /// The oracle eigenda provider.
    eigenda_provider: OracleEigenDAProvider<T>,
}

impl<T: CommsClient> OracleAltDAProvider<T> {
    /// Constructs a new oracle-backed AltDA provider.
    pub fn new(eigenda_provider: OracleEigenDAProvider<T>) -> Self {
        Self { eigenda_provider }
    }

    /// Constructs a new oracle-backed AltDA provider by constructing
    /// the respective altda providers using the oracle.
    pub fn new_from_oracle(oracle: Arc<T>) -> Self {
        Self { eigenda_provider: OracleEigenDAProvider::new(oracle) }
    }
}

#[async_trait]
impl<T: CommsClient + Send + Sync> AltDAProvider for OracleAltDAProvider<T> {
    type Error = OracleProviderError;
    /// Retrieves a blob from the oracle.
    ///
    /// ## Takes
    /// - `commitment`: The commitment to the blob (specific to each AltDA provider).
    ///
    /// ## Returns
    /// - `Ok(Bytes)`: The blob.
    /// - `Err(e)`: The blob could not be retrieved.
    async fn get_blob(&self, commitment: AltDACommitment) -> Result<Bytes, OracleProviderError> {
        match commitment {
            AltDACommitment::Keccak(_) => Err(OracleProviderError::AltDA(
                "keccak commitments are not implemented yet".to_string(),
            )),
            AltDACommitment::EigenDAV1(cert) => self.eigenda_provider.get_blob_v1(cert).await,
            AltDACommitment::EigenDAV2(cert) => self.eigenda_provider.get_blob_v2(cert).await,
            AltDACommitment::Avail(_) => Err(OracleProviderError::AltDA(
                "avail commitments are not implemented yet".to_string(),
            )),
            AltDACommitment::Celestia(_) => Err(OracleProviderError::AltDA(
                "celestia commitments are not implemented yet".to_string(),
            )),
        }
    }
}

 */