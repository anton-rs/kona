//! Contains the [TrieDBFetcher] trait for fetching trie node preimages, contract bytecode, and
//! headers.

use alloy_consensus::Header;
use alloy_primitives::{Bytes, B256};
use anyhow::Result;

/// The [TrieDBFetcher] trait defines the synchronous interface for fetching trie node preimages and
/// headers.
pub trait TrieDBFetcher {
    /// Fetches the preimage for the given trie node hash.
    ///
    /// ## Takes
    /// - `key`: The key of the trie node to fetch.
    ///
    /// ## Returns
    /// - Ok(Bytes): The trie node preimage.
    /// - Err(anyhow::Error): If the trie node preimage could not be fetched.
    ///
    /// [TrieDB]: crate::TrieDB
    fn trie_node_preimage(&self, key: B256) -> Result<Bytes>;

    /// Fetches the preimage of the bytecode hash provided.
    ///
    /// ## Takes
    /// - `hash`: The hash of the bytecode.
    ///
    /// ## Returns
    /// - Ok(Bytes): The bytecode of the contract.
    /// - Err(anyhow::Error): If the bytecode hash could not be fetched.
    ///
    /// [TrieDB]: crate::TrieDB
    fn bytecode_by_hash(&self, code_hash: B256) -> Result<Bytes>;

    /// Fetches the preimage of [Header] hash provided.
    ///
    /// ## Takes
    /// - `hash`: The hash of the RLP-encoded [Header].
    ///
    /// ## Returns
    /// - Ok(Bytes): The [Header].
    /// - Err(anyhow::Error): If the [Header] could not be fetched.
    ///
    /// [TrieDB]: crate::TrieDB
    fn header_by_hash(&self, hash: B256) -> Result<Header>;
}

/// The default, no-op implementation of the [TrieDBFetcher] trait, used for testing.
#[derive(Debug, Clone, Copy)]
pub struct NoopTrieDBFetcher;

impl TrieDBFetcher for NoopTrieDBFetcher {
    fn trie_node_preimage(&self, _key: B256) -> Result<Bytes> {
        Ok(Bytes::new())
    }

    fn bytecode_by_hash(&self, _code_hash: B256) -> Result<Bytes> {
        Ok(Bytes::new())
    }

    fn header_by_hash(&self, _hash: B256) -> Result<Header> {
        Ok(Header::default())
    }
}
