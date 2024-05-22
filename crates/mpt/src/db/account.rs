//! This module contains the [TrieAccount] struct.

use alloy_primitives::{B256, U256};
use alloy_rlp::{RlpDecodable, RlpEncodable};
use revm::primitives::{Account, AccountInfo};

/// An Ethereum account as represented in the trie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
pub struct TrieAccount {
    /// Account nonce.
    pub nonce: u64,
    /// Account balance.
    pub balance: U256,
    /// Account's storage root.
    pub storage_root: B256,
    /// Hash of the account's bytecode.
    pub code_hash: B256,
}

impl From<(Account, B256)> for TrieAccount {
    fn from((account, storage_root): (Account, B256)) -> Self {
        Self {
            nonce: account.info.nonce,
            balance: account.info.balance,
            storage_root,
            code_hash: account.info.code_hash,
        }
    }
}

impl From<(AccountInfo, B256)> for TrieAccount {
    fn from((account, storage_root): (AccountInfo, B256)) -> Self {
        Self {
            nonce: account.nonce,
            balance: account.balance,
            storage_root,
            code_hash: account.code_hash,
        }
    }
}
