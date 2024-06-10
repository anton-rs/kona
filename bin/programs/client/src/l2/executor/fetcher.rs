//! Contains the fetcher construction functions for the block executor's [TrieDB].
//!
//! [TrieDB]: kona_mpt::TrieDB

use crate::CachingOracle;
use alloy_consensus::Header;
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use anyhow::{anyhow, Result};
use kona_mpt::TrieDBFetcher;
use kona_preimage::{PreimageKey, PreimageKeyType, PreimageOracleClient};

/// The [TrieDBFetcher] implementation for the block executor's [TrieDB].
///
/// TODO: Move this into the higher-level L2 chain fetcher, and also implement the [TrieDBFetcher]
/// trait.
///
/// [TrieDB]: kona_mpt::TrieDB
#[derive(Debug)]
pub struct TrieDBProvider<'a, const N: usize> {
    /// The inner caching oracle to fetch trie node preimages from.
    caching_oracle: &'a CachingOracle<N>,
}

impl<'a, const N: usize> TrieDBProvider<'a, N> {
    /// Constructs a new [TrieDBProvider] with the given [CachingOracle].
    pub fn new(caching_oracle: &'a CachingOracle<N>) -> Self {
        Self { caching_oracle }
    }
}

impl<'a, const N: usize> TrieDBFetcher for TrieDBProvider<'a, N> {
    fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
        // Fetch the trie preimage from the caching oracle.
        kona_common::block_on(async move {
            self.caching_oracle
                .get(PreimageKey::new(*key, PreimageKeyType::Keccak256))
                .await
                .map(Into::into)
        })
    }

    fn bytecode_by_hash(&self, _: B256) -> Result<Bytes> {
        todo!()
    }

    fn header_by_hash(&self, hash: B256) -> Result<Header> {
        // Fetch the header from the caching oracle.
        kona_common::block_on(async move {
            let header_bytes = self
                .caching_oracle
                .get(PreimageKey::new(*hash, PreimageKeyType::Keccak256))
                .await?;
            Header::decode(&mut header_bytes.as_slice())
                .map_err(|e| anyhow!("Failed to RLP decode Header: {e}"))
        })
    }
}
