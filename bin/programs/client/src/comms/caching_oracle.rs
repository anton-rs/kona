//! Contains the [CachingOracle], which is a wrapper around an [OracleReader] that stores a
//! configurable number of responses in an [LruCache] for quick retrieval.
//!
//! [OracleReader]: kona_preimage::OracleReader

use crate::ORACLE_READER;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use anyhow::Result;
use async_trait::async_trait;
use core::num::NonZeroUsize;
use kona_preimage::{PreimageKey, PreimageOracleClient};
use lru::LruCache;
use spin::Mutex;

/// A wrapper around an [OracleReader] that stores a configurable number of responses in an
/// [LruCache] for quick retrieval.
///
/// [OracleReader]: kona_preimage::OracleReader
#[derive(Debug, Clone)]
pub struct CachingOracle<const N: usize> {
    /// The spin-locked cache that stores the responses from the oracle.
    cache: Arc<Mutex<LruCache<PreimageKey, Vec<u8>>>>,
}

impl<const N: usize> CachingOracle<N> {
    /// Creates a new [CachingOracle] that wraps the given [OracleReader] and stores up to `N`
    /// responses in the cache.
    ///
    /// [OracleReader]: kona_preimage::OracleReader
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(N).expect("N must be greater than 0"),
            ))),
        }
    }
}

impl<const N: usize> Default for CachingOracle<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<const N: usize> PreimageOracleClient for CachingOracle<N> {
    async fn get(&self, key: PreimageKey) -> Result<Vec<u8>> {
        let mut cache_lock = self.cache.lock();
        if let Some(value) = cache_lock.get(&key) {
            Ok(value.clone())
        } else {
            let value = ORACLE_READER.get(key).await?;
            cache_lock.put(key, value.clone());
            Ok(value)
        }
    }

    async fn get_exact(&self, key: PreimageKey, buf: &mut [u8]) -> Result<()> {
        let mut cache_lock = self.cache.lock();
        if let Some(value) = cache_lock.get(&key) {
            // SAFETY: The value never enters the cache unless the preimage length matches the
            // buffer length, due to the checks in the OracleReader.
            buf.copy_from_slice(value.as_slice());
            Ok(())
        } else {
            ORACLE_READER.get_exact(key, buf).await?;
            cache_lock.put(key, buf.to_vec());
            Ok(())
        }
    }
}
