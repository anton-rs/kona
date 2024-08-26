//! [ExecutionFixture]-backed trie fetcher implementation.

use alloy_consensus::{Header, EMPTY_ROOT_HASH};
use alloy_primitives::{b256, keccak256, Address, Bytes, B256, U256};
use alloy_rlp::Encodable;
use alloy_trie::{proof::ProofRetainer, HashBuilder, Nibbles};
use anyhow::{anyhow, Result};
use itertools::Itertools;
use kona_mpt::{TrieAccount, TrieDBFetcher, TrieDBHinter};
use op_test_vectors::execution::ExecutionFixture;
use std::collections::HashMap;

const EMPTY_HASH: B256 = b256!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");

#[derive(Debug, Clone)]
pub(crate) struct ExecutionFixtureTrieFetcher<'a> {
    /// The innner [ExecutionFixture] instance.
    pub(crate) fixture: &'a ExecutionFixture,
    /// Current state root.
    pub(crate) root: B256,
    /// The cache of the trie nodes.
    pub(crate) preimages: HashMap<B256, Bytes>,
}

impl<'a> ExecutionFixtureTrieFetcher<'a> {
    /// Create a new [ExecutionFixtureTrieFetcher] instance.
    pub(crate) fn new(fixture: &'a ExecutionFixture) -> Result<Self> {
        let (root, preimages) = state_trie_witness(fixture)?;
        Ok(Self { fixture, root, preimages })
    }
}

impl<'a> TrieDBFetcher for ExecutionFixtureTrieFetcher<'a> {
    fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
        self.preimages.get(&key).cloned().ok_or(anyhow!("Missing trie node preimage"))
    }

    fn bytecode_by_hash(&self, code_hash: B256) -> Result<Bytes> {
        Ok(self
            .fixture
            .alloc
            .iter()
            .find_map(|(_, account)| {
                (account.code.as_ref().map(keccak256) == Some(code_hash))
                    .then(|| account.code.clone())
                    .flatten()
            })
            .unwrap_or_default())
    }

    fn header_by_hash(&self, _: B256) -> Result<Header> {
        unimplemented!()
    }
}

impl<'a> TrieDBHinter for ExecutionFixtureTrieFetcher<'a> {
    fn hint_account_proof(&self, _address: Address, _block_number: u64) -> Result<()> {
        Ok(())
    }

    fn hint_storage_proof(&self, _address: Address, _slot: U256, _block_number: u64) -> Result<()> {
        Ok(())
    }

    fn hint_trie_node(&self, _hash: B256) -> Result<()> {
        Ok(())
    }
}

/// Computes the state trie for the prestate of the given [ExecutionFixture], and returns a map of
/// the trie node hashes to their corresponding RLP-encoded values.
fn state_trie_witness(fixture: &ExecutionFixture) -> Result<(B256, HashMap<B256, Bytes>)> {
    let mut cache = HashMap::new();

    // First, generate all trie accounts.
    let mut trie_accounts = HashMap::new();
    for (address, account_state) in fixture.alloc.iter() {
        let storage_root = if let Some(storage) = &account_state.storage {
            let slot_nibbles =
                storage.keys().map(|k| Nibbles::unpack(keccak256(k))).collect::<Vec<_>>();
            let mut hb =
                HashBuilder::default().with_proof_retainer(ProofRetainer::new(slot_nibbles));

            let sorted_storage = storage
                .iter()
                .filter(|(_, v)| **v != B256::ZERO)
                .sorted_by_key(|(k, _)| keccak256(k.as_slice()));
            for (slot, value) in sorted_storage {
                let slot_nibbles = Nibbles::unpack(keccak256(slot.as_slice()));
                let mut value_buf = Vec::with_capacity(value.length());
                U256::from_be_slice(value.as_slice()).encode(&mut value_buf);
                hb.add_leaf(slot_nibbles, &value_buf);
            }

            // Compute the root of the account storage trie.
            let root = hb.root();

            // Insert the proofs into the global cache.
            let proofs = hb.take_proofs();
            for (_, node) in proofs {
                let val_hash = keccak256(&node);
                cache.insert(val_hash, node);
            }

            root
        } else {
            EMPTY_ROOT_HASH
        };

        // Construct the account trie.
        let trie_account = TrieAccount {
            nonce: account_state.nonce.unwrap_or_default(),
            balance: account_state.balance,
            storage_root,
            code_hash: account_state.code.as_ref().map(keccak256).unwrap_or(EMPTY_HASH),
        };

        trie_accounts.insert(address, trie_account);
    }

    let hashed_address_nibbles = fixture
        .alloc
        .keys()
        .map(|addr| Nibbles::unpack(keccak256(addr.as_slice())))
        .collect::<Vec<_>>();
    let mut hb =
        HashBuilder::default().with_proof_retainer(ProofRetainer::new(hashed_address_nibbles));

    // Insert all trie accounts into the state trie.
    let sorted_accounts = trie_accounts.iter().sorted_by_key(|(k, _)| keccak256(k));
    for (address, _) in sorted_accounts {
        let trie_account = trie_accounts.get(address).ok_or(anyhow!("Missing trie account"))?;

        let address_nibbles = Nibbles::unpack(keccak256(address));
        let mut account_buffer = Vec::with_capacity(trie_account.length());
        trie_account.encode(&mut account_buffer);
        hb.add_leaf(address_nibbles, &account_buffer);
    }

    // Compute the root of the state trie.
    let root = hb.root();

    // Insert the proofs into the global cache.
    let proofs = hb.take_proofs();
    for (_, node) in proofs {
        let val_hash = keccak256(&node);
        cache.insert(val_hash, node);
    }

    Ok((root, cache))
}
