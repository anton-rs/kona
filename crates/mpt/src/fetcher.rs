//! Contains the [TrieDBFetcher] trait for fetching trie node preimages, contract bytecode, and
//! headers.

use alloc::string::{String, ToString};
use alloy_consensus::Header;
use alloy_primitives::{Address, Bytes, B256, U256};
use core::fmt::Display;

/// The [TrieDBFetcher] trait defines the synchronous interface for fetching trie node preimages and
/// headers.
pub trait TrieDBFetcher {
    /// The error type for fetching trie node preimages.
    type Error: Display + ToString;

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
    fn trie_node_preimage(&self, key: B256) -> Result<Bytes, Self::Error>;

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
    fn bytecode_by_hash(&self, code_hash: B256) -> Result<Bytes, Self::Error>;

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
    fn header_by_hash(&self, hash: B256) -> Result<Header, Self::Error>;
}

/// The [TrieDBHinter] trait defines the synchronous interface for hinting the host to fetch trie
/// node preimages.
pub trait TrieDBHinter {
    /// The error type for hinting trie node preimages.
    type Error: Display + ToString;

    /// Hints the host to fetch the trie node preimage by hash.
    ///
    /// ## Takes
    /// - `hash`: The hash of the trie node to hint.
    ///
    /// ## Returns
    /// - Ok(()): If the hint was successful.
    fn hint_trie_node(&self, hash: B256) -> Result<(), Self::Error>;

    /// Hints the host to fetch the trie node preimages on the path to the given address.
    ///
    /// ## Takes
    /// - `address` - The address of the contract whose trie node preimages are to be fetched.
    /// - `block_number` - The block number at which the trie node preimages are to be fetched.
    ///
    /// ## Returns
    /// - Ok(()): If the hint was successful.
    /// - Err(anyhow::Error): If the hint was unsuccessful.
    fn hint_account_proof(&self, address: Address, block_number: u64) -> Result<(), Self::Error>;

    /// Hints the host to fetch the trie node preimages on the path to the storage slot within the
    /// given account's storage trie.
    ///
    /// ## Takes
    /// - `address` - The address of the contract whose trie node preimages are to be fetched.
    /// - `slot` - The storage slot whose trie node preimages are to be fetched.
    /// - `block_number` - The block number at which the trie node preimages are to be fetched.
    ///
    /// ## Returns
    /// - Ok(()): If the hint was successful.
    /// - Err(anyhow::Error): If the hint was unsuccessful.
    fn hint_storage_proof(
        &self,
        address: Address,
        slot: U256,
        block_number: u64,
    ) -> Result<(), Self::Error>;
}

/// The default, no-op implementation of the [TrieDBFetcher] trait, used for testing.
#[derive(Debug, Clone, Copy)]
pub struct NoopTrieDBFetcher;

impl TrieDBFetcher for NoopTrieDBFetcher {
    type Error = String;

    fn trie_node_preimage(&self, _key: B256) -> Result<Bytes, Self::Error> {
        Ok(Bytes::new())
    }

    fn bytecode_by_hash(&self, _code_hash: B256) -> Result<Bytes, Self::Error> {
        Ok(Bytes::new())
    }

    fn header_by_hash(&self, _hash: B256) -> Result<Header, Self::Error> {
        Ok(Header::default())
    }
}

/// The default, no-op implementation of the [TrieDBHinter] trait, used for testing.
#[derive(Debug, Clone, Copy)]
pub struct NoopTrieDBHinter;

impl TrieDBHinter for NoopTrieDBHinter {
    type Error = String;

    fn hint_trie_node(&self, _hash: B256) -> Result<(), Self::Error> {
        Ok(())
    }

    fn hint_account_proof(&self, _address: Address, _block_number: u64) -> Result<(), Self::Error> {
        Ok(())
    }

    fn hint_storage_proof(
        &self,
        _address: Address,
        _slot: U256,
        _block_number: u64,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
