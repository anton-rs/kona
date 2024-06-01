//! This module contains an implementation of an in-memory Trie DB for [revm], that allows for
//! incremental updates through fetching node preimages on the fly during execution.

use crate::TrieNode;
use alloc::vec::Vec;
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_rlp::{Decodable, Encodable};
use alloy_trie::Nibbles;
use anyhow::{anyhow, Result};
use revm::{
    db::BundleState,
    primitives::{AccountInfo, Bytecode, HashMap},
    Database,
};

mod account;
pub use account::TrieAccount;

/// A Trie DB that caches open state in-memory. When accounts that don't already exist within the
/// cached [TrieNode] are queried, the database fetches the preimages of the trie nodes on the path
/// to the account using the `PreimageFetcher` (`PF` generic) and `CodeHashFetcher` (`CHF` generic).
/// This allows for data to be fetched in a verifiable manner given an initial trusted state root
/// as it is needed during execution.
///
/// The [TrieDB] is intended to be wrapped by a [State], which is then used by the [revm::Evm] to
/// capture state transitions during block execution.
///
/// **Behavior**:
/// - When an account is queried and the trie path has not already been opened by [Self::basic], we
///   fall through to the `PreimageFetcher` to fetch the preimages of the trie nodes on the path to
///   the account. After it has been fetched, the path will be cached until the next call to
///   [Self::state_root].
/// - When querying for the code hash of an account, the `CodeHashFetcher` is consulted to fetch the
///   code hash of the account.
/// - When a [BundleState] changeset is committed to the parent [State] database, the changes are
///   first applied to the [State]'s cache, then the trie hash is recomputed with
///   [Self::state_root].
///
/// **Example Construction**:
/// ```rust
/// use alloy_primitives::{Bytes, B256};
/// use anyhow::Result;
/// use kona_mpt::TrieDB;
/// use revm::{db::states::bundle_state::BundleRetention, EvmBuilder, StateBuilder};
///
/// let mock_fetcher = |hash: B256| -> Result<Bytes> { Ok(Default::default()) };
/// let mock_starting_root = B256::default();
///
/// let trie_db = TrieDB::new(mock_starting_root, mock_fetcher, mock_fetcher);
/// let mut state = StateBuilder::new_with_database(trie_db).with_bundle_update().build();
/// let evm = EvmBuilder::default().with_db(&mut state).build();
///
/// // Execute your block's transactions...
///
/// // Drop the EVM prior to merging the state transitions.
/// drop(evm);
///
/// state.merge_transitions(BundleRetention::PlainState);
/// let bundle = state.take_bundle();
/// let state_root = state.database.state_root(&bundle).expect("Failed to compute state root");
/// ```
///
/// [State]: revm::State
#[derive(Debug, Clone)]
pub struct TrieDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(B256) -> Result<Bytes> + Copy,
{
    /// The [TrieNode] representation of the root node.
    root_node: TrieNode,
    /// Storage roots of accounts within the trie.
    storage_roots: HashMap<Address, TrieNode>,
    /// The preimage fetching function
    preimage_fetcher: PF,
    /// The code hash fetching function
    code_by_hash_fetcher: CHF,
}

impl<PF, CHF> TrieDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(B256) -> Result<Bytes> + Copy,
{
    /// Creates a new [TrieDB] with the given root node.
    pub fn new(root: B256, preimage_fetcher: PF, code_by_hash_fetcher: CHF) -> Self {
        Self {
            root_node: TrieNode::new_blinded(root),
            preimage_fetcher,
            code_by_hash_fetcher,
            storage_roots: Default::default(),
        }
    }

    /// Consumes `Self` and takes the the current state root of the trie DB.
    pub fn take_root_node(self) -> TrieNode {
        self.root_node
    }

    /// Returns a shared reference to the root [TrieNode] of the trie DB.
    pub fn root_node_ref(&self) -> &TrieNode {
        &self.root_node
    }

    /// Returns a mutable reference to the root [TrieNode] of the trie DB.
    ///
    /// # Safety
    /// This method is unsafe because it allows for the mutation of the root node, which enables
    /// the caller to mutate the [TrieNode] of the DB without updating the root hash. This can lead
    /// to inconsistencies in the trie DB's state. The caller must ensure that the root hash is
    /// updated after mutating the root node.
    pub unsafe fn root_node_mut(&mut self) -> &mut TrieNode {
        &mut self.root_node
    }

    /// Returns the mapping of [Address]es to storage roots.
    pub fn storage_roots(&self) -> &HashMap<Address, TrieNode> {
        &self.storage_roots
    }

    /// Returns the mapping of [Address]es to storage roots.
    ///
    /// # Safety
    /// This method is unsafe because it allows for the mutation of the storage roots, which enables
    /// the caller to mutate the [TrieNode] of an account in the DB without updating the root hash
    /// or validating the account storage root within the state trie. The caller must ensure
    /// that any changes to the storage roots are consistent with the state trie.
    pub unsafe fn storage_roots_mut(&mut self) -> &mut HashMap<Address, TrieNode> {
        &mut self.storage_roots
    }

    /// Applies a [BundleState] changeset to the [TrieNode] and recomputes the state root hash.
    ///
    /// # Takes
    /// - `bundle`: The [BundleState] changeset to apply to the trie DB.
    ///
    /// # Returns
    /// - `Ok(B256)`: The new state root hash of the trie DB.
    /// - `Err(_)`: If the state root hash could not be computed.
    pub fn state_root(&mut self, bundle: &BundleState) -> Result<B256> {
        // Update the accounts in the trie with the changeset.
        self.update_accounts(bundle)?;

        // Recompute the root hash of the trie.
        self.root_node.blind();

        // Extract the new state root from the root node.
        self.root_node.blinded_commitment().ok_or(anyhow!("State root node is not a blinded node"))
    }

    /// Modifies the accounts in the storage trie with the given [BundleState] changeset.
    ///
    /// # Takes
    /// - `bundle`: The [BundleState] changeset to apply to the trie DB.
    ///
    /// # Returns
    /// - `Ok(())` if the accounts were successfully updated.
    /// - `Err(_)` if the accounts could not be updated.
    fn update_accounts(&mut self, bundle: &BundleState) -> Result<()> {
        for (address, bundle_account) in bundle.state() {
            let account_info =
                bundle_account.account_info().ok_or(anyhow!("Account info not found"))?;
            let mut trie_account = TrieAccount {
                balance: account_info.balance,
                nonce: account_info.nonce,
                code_hash: account_info.code_hash,
                ..Default::default()
            };

            // Update the account's storage root
            let acc_storage_root = self
                .storage_roots
                .get_mut(address)
                .ok_or(anyhow!("Storage root not found for account"))?;
            bundle_account.storage.iter().try_for_each(|(index, value)| {
                Self::change_storage(
                    acc_storage_root,
                    *index,
                    value.present_value,
                    self.preimage_fetcher,
                )
            })?;

            // Recompute the account storage root.
            acc_storage_root.blind();

            let commitment = acc_storage_root
                .blinded_commitment()
                .ok_or(anyhow!("Storage root node is not a blinded node"))?;
            trie_account.storage_root = commitment;

            // RLP encode the trie account for insertion.
            let mut account_buf = Vec::with_capacity(trie_account.length());
            trie_account.encode(&mut account_buf);

            // Insert or update the account in the trie.
            let account_path = Nibbles::unpack(keccak256(address.as_slice()));
            if let Ok(account_rlp_ref) = self.root_node.open(&account_path, self.preimage_fetcher) {
                // Update the existing account in the trie.
                *account_rlp_ref = account_buf.into();
            } else {
                // Insert the new account into the trie.
                self.root_node.insert(&account_path, account_buf.into(), self.preimage_fetcher)?;
            }
        }

        Ok(())
    }

    /// Modifies a storage slot of an account in the Merkle Patricia Trie.
    ///
    /// # Takes
    /// - `address`: The address of the account.
    /// - `index`: The index of the storage slot.
    /// - `value`: The new value of the storage slot.
    ///
    /// # Returns
    /// - `Ok(())` if the storage slot was successfully modified.
    /// - `Err(_)` if the storage slot could not be modified.
    fn change_storage(
        storage_root: &mut TrieNode,
        index: U256,
        value: U256,
        preimage_fetcher: PF,
    ) -> Result<()> {
        // RLP encode the storage slot value.
        let mut rlp_buf = Vec::with_capacity(value.length());
        value.encode(&mut rlp_buf);

        // Insert or update the storage slot in the trie.
        let hashed_slot_key = keccak256(index.to_be_bytes::<32>().as_slice());
        if let Ok(storage_slot_rlp) =
            storage_root.open(&Nibbles::unpack(hashed_slot_key), preimage_fetcher)
        {
            // If the storage slot already exists, update it.
            *storage_slot_rlp = rlp_buf.into();
        } else {
            // If the storage slot does not exist, insert it.
            storage_root.insert(
                &Nibbles::unpack(hashed_slot_key),
                rlp_buf.into(),
                preimage_fetcher,
            )?;
        }

        Ok(())
    }
}

impl<PF, CHF> Database for TrieDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(B256) -> Result<Bytes> + Copy,
{
    type Error = anyhow::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        // Fetch the account from the trie.
        let hashed_address_nibbles = Nibbles::unpack(keccak256(address.as_slice()));
        let Ok(trie_account_rlp) =
            self.root_node.open(&hashed_address_nibbles, self.preimage_fetcher)
        else {
            // If the account does not exist in the trie, return `Ok(None)`.
            return Ok(None);
        };

        // Decode the trie account from the RLP bytes.
        let trie_account = TrieAccount::decode(&mut trie_account_rlp.as_ref())
            .map_err(|e| anyhow!("Error decoding trie account: {e}"))?;

        // Insert the account's storage root into the cache.
        self.storage_roots.insert(address, TrieNode::new_blinded(trie_account.storage_root));

        // Return a partial DB account. The storage and code are not loaded out-right, and are
        // loaded optimistically in the `Database` + `DatabaseRef` trait implementations.
        Ok(Some(AccountInfo {
            balance: trie_account.balance,
            nonce: trie_account.nonce,
            code_hash: trie_account.code_hash,
            code: None,
        }))
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        (self.code_by_hash_fetcher)(code_hash)
            .map(Bytecode::new_raw)
            .map_err(|e| anyhow!("Failed to fetch code by hash: {e}"))
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        // Fetch the account's storage root from the cache. If storage is being accessed, the
        // account should have been loaded into the cache by the `basic` method.
        let storage_root = self
            .storage_roots
            .get_mut(&address)
            .ok_or(anyhow!("Storage root not found for account {address}"))?;

        // Fetch the storage slot from the trie.
        let hashed_slot_key = keccak256(index.to_be_bytes::<32>().as_slice());
        let slot_value =
            storage_root.open(&Nibbles::unpack(hashed_slot_key), self.preimage_fetcher)?;

        // Decode the storage slot value.
        let int_slot = U256::decode(&mut slot_value.as_ref())
            .map_err(|e| anyhow!("Failed to decode storage slot value: {e}"))?;

        Ok(int_slot)
    }

    fn block_hash(&mut self, _: U256) -> Result<B256, Self::Error> {
        // match self.db.block_hashes.entry(number) {
        //     Entry::Occupied(entry) => Ok(*entry.get()),
        //     Entry::Vacant(_) => anyhow::bail!("Block hash for number not found"),
        // }
        unimplemented!("Block hash not implemented; Need to unroll the starting block hash for this operation.")
    }
}
