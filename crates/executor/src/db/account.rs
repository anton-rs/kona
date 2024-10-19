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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::uint;

    #[test]
    fn test_trie_account_from_account() {
        let account = Account {
            info: AccountInfo {
                nonce: 1,
                balance: uint!(2_U256),
                code_hash: B256::default(),
                code: Default::default(),
            },
            status: Default::default(),
            storage: Default::default(),
        };
        let storage_root = B256::default();
        let trie_account = TrieAccount::from((account, storage_root));
        assert_eq!(trie_account.nonce, 1);
        assert_eq!(trie_account.balance, uint!(2_U256));
        assert_eq!(trie_account.storage_root, B256::default());
        assert_eq!(trie_account.code_hash, B256::default());
    }

    #[test]
    fn test_trie_account_from_account_info() {
        let account_info = AccountInfo {
            nonce: 1,
            balance: uint!(2_U256),
            code_hash: B256::default(),
            code: Default::default(),
        };
        let storage_root = B256::default();
        let trie_account = TrieAccount::from((account_info, storage_root));
        assert_eq!(trie_account.nonce, 1);
        assert_eq!(trie_account.balance, uint!(2_U256));
        assert_eq!(trie_account.storage_root, B256::default());
        assert_eq!(trie_account.code_hash, B256::default());
    }
}
