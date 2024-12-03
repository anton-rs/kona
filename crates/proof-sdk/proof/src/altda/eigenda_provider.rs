use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloy_primitives::{keccak256, Bytes};
use async_trait::async_trait;
use kona_derive::traits::EigenDABlobProvider;
use kona_preimage::{CommsClient, PreimageKey, PreimageKeyType};

use crate::errors::OracleProviderError;
use crate::HintType;

/// The oracle-backed EigenDA provider for the client program.
#[derive(Debug, Clone)]
pub struct OracleEigenDAProvider<T: CommsClient> {
    /// The preimage oracle client.
    oracle: Arc<T>,
}

impl<T: CommsClient> OracleEigenDAProvider<T> {
    /// Constructs a new oracle-backed EigenDA provider.
    pub fn new(oracle: Arc<T>) -> Self {
        Self { oracle }
    }
}

#[async_trait]
impl<T: CommsClient + Sync + Send> EigenDABlobProvider for OracleEigenDAProvider<T> {
    type Error = OracleProviderError;

    async fn get_blob(&self, cert: Bytes) -> Result<Bytes, Self::Error> {
        Err(OracleProviderError::AltDA("eigenda v1 ncommented".to_string()))
        /*
        self.oracle
            .write(&HintType::AltDACommitment.encode_with(&[&cert]))
            .await
            .map_err(OracleProviderError::Preimage)?;
        let data = self
            .oracle
            .get(PreimageKey::new(*keccak256(cert), PreimageKeyType::GlobalGeneric))
            .await
            .map_err(OracleProviderError::Preimage)?;
        Ok(data.into())
        */
    }
}