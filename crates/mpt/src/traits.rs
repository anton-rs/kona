//! Contains the [TrieProvider] trait for fetching trie node preimages, contract bytecode, and
//! headers.

use crate::TrieNode;
use alloc::string::ToString;
use alloy_consensus::Header;
use alloy_primitives::{Address, Bytes, B256, U256};
use core::fmt::Display;

/// The [TrieProvider] trait defines the synchronous interface for fetching trie node preimages and
/// headers.
pub trait TrieProvider {
    /// The error type for fetching trie node preimages.
    type Error: Display + ToString;

    /// Fetches the preimage for the given trie node hash.
    ///
    /// ## Takes
    /// - `key`: The key of the trie node to fetch.
    ///
    /// ## Returns
    /// - Ok(TrieNode): The trie node preimage.
    /// - Err(anyhow::Error): If the trie node preimage could not be fetched.
    ///
    /// [TrieDB]: crate::TrieDB
    fn trie_node_by_hash(&self, key: B256) -> Result<TrieNode, Self::Error>;

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

/// The [TrieHinter] trait defines the synchronous interface for hinting the host to fetch trie
/// node preimages.
pub trait TrieHinter {
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
