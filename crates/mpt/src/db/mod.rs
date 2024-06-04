//! This module contains an implementation of an in-memory Trie DB for [revm], that allows for
//! incremental updates through fetching node preimages on the fly during execution.

use crate::TrieNode;
use alloc::vec::Vec;
use alloy_consensus::constants::KECCAK_EMPTY;
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_rlp::{Decodable, Encodable};
use alloy_trie::Nibbles;
use anyhow::{anyhow, Result};
use revm::{
    db::{AccountState, DbAccount},
    primitives::{hash_map::Entry, Account, AccountInfo, Bytecode, HashMap},
    Database, DatabaseCommit, InMemoryDB,
};
use tracing::trace;

mod account;
pub use account::TrieAccount;

/// A Trie DB that caches account state in-memory. When accounts that don't already exist within the
/// cache are queried, the database fetches the preimages of the trie nodes on the path to the
/// account using the `PreimageFetcher` (`PF` generic) and `CodeHashFetcher` (`CHF` generic). This
/// allows for data to be fetched in a verifiable manner given an initial trusted state root as it
/// is needed during execution.
///
/// **Behavior**:
/// - When an account is queried and it does not already exist in the inner cache database, we fall
///   through to the `PreimageFetcher` to fetch the preimages of the trie nodes on the path to the
///   account. After it has been fetched, the account is inserted into the cache database and will
///   be read from there on subsequent queries.
/// - When querying for the code hash of an account, the `CodeHashFetcher` is consulted to fetch the
///   code hash of the account.
/// - When a changeset is committed to the database, the changes are first applied to the cache
///   database and then the trie hash is recomputed. The root hash of the trie is then persisted to
///   the struct.
///
/// Note: This Database implementation intentionally wraps the [InMemoryDB], rather than serving as
/// a backing database for [revm::db::CacheDB]. This is because the [revm::db::CacheDB] is designed
/// to be a cache layer on top of a [revm::DatabaseRef] implementation, and the [TrieCacheDB] has a
/// requirement to also cache the opened state trie and account storage trie nodes, which requires
/// mutable access.
#[derive(Debug, Clone)]
pub struct TrieCacheDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(B256) -> Result<Bytes> + Copy,
{
    /// The underlying DB that stores the account state in-memory.
    db: InMemoryDB,
    /// The root hash of the trie.
    root: B256,
    /// The [TrieNode] representation of the root node.
    root_node: TrieNode,
    /// Storage roots of accounts within the trie.
    storage_roots: HashMap<Address, TrieNode>,
    /// The preimage fetching function
    preimage_fetcher: PF,
    /// The code hash fetching function
    code_by_hash_fetcher: CHF,
}

impl<PF, CHF> TrieCacheDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(B256) -> Result<Bytes> + Copy,
{
    /// Creates a new [TrieCacheDB] with the given root node.
    pub fn new(root: B256, preimage_fetcher: PF, code_by_hash_fetcher: CHF) -> Self {
        Self {
            db: InMemoryDB::default(),
            root,
            root_node: TrieNode::Blinded { commitment: root },
            preimage_fetcher,
            code_by_hash_fetcher,
            storage_roots: Default::default(),
        }
    }

    /// Returns a reference to the underlying in-memory DB.
    pub fn inner_db_ref(&self) -> &InMemoryDB {
        &self.db
    }

    /// Returns a mutable reference to the underlying in-memory DB.
    pub fn inner_db_mut(&mut self) -> &mut InMemoryDB {
        &mut self.db
    }

    /// Returns the current state root of the trie DB, and replaces the root node with the new
    /// blinded form. This action drops all the cached account state.
    pub fn state_root(&mut self) -> Result<B256> {
        trace!("Start state root update");
        self.root_node.blind();
        trace!("State root node updated successfully");

        let commitment = if let TrieNode::Blinded { commitment } = self.root_node {
            commitment
        } else {
            anyhow::bail!("Root node is not a blinded node")
        };

        self.root = commitment;
        Ok(commitment)
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

    /// Loads an account from the trie by consulting the `PreimageFetcher` to fetch the preimages of
    /// the trie nodes on the path to the account. If the account has a non-empty storage trie
    /// root hash, the account's storage trie will be traversed to recover the account's storage
    /// slots. If the account has a non-empty
    ///
    /// # Takes
    /// - `address`: The address of the account to load.
    ///
    /// # Returns
    /// - `Ok(DbAccount)`: The account loaded from the trie.
    /// - `Err(_)`: If the account could not be loaded from the trie.
    pub(crate) fn load_account_from_trie(&mut self, address: Address) -> Result<DbAccount> {
        let hashed_address_nibbles = Nibbles::unpack(keccak256(address.as_slice()));
        let trie_account_rlp =
            self.root_node.open(&hashed_address_nibbles, self.preimage_fetcher)?;
        let trie_account = TrieAccount::decode(&mut trie_account_rlp.as_ref())
            .map_err(|e| anyhow!("Error decoding trie account: {e}"))?;

        // Insert the account's storage root into the cache.
        self.storage_roots
            .insert(address, TrieNode::Blinded { commitment: trie_account.storage_root });

        // If the account's code hash is not empty, fetch the bytecode and insert it into the cache.
        let code = (trie_account.code_hash != KECCAK_EMPTY)
            .then(|| {
                let code = Bytecode::new_raw((self.code_by_hash_fetcher)(trie_account.code_hash)?);
                Ok::<_, anyhow::Error>(code)
            })
            .transpose()?;

        // Return a partial DB account. The storage and code are not loaded out-right, and are
        // loaded optimistically in the `Database` + `DatabaseRef` trait implementations.
        let mut info = AccountInfo {
            balance: trie_account.balance,
            nonce: trie_account.nonce,
            code_hash: trie_account.code_hash,
            code,
        };
        self.insert_contract(&mut info);

        Ok(DbAccount { info, ..Default::default() })
    }

    /// Inserts the account's code into the cache.
    ///
    /// Accounts objects and code are stored separately in the cache, this will take the code from
    /// the account and instead map it to the code hash.
    ///
    /// # Takes
    /// - `account`: The account to insert the code for.
    pub(crate) fn insert_contract(&mut self, account: &mut AccountInfo) {
        if let Some(code) = &account.code {
            if !code.is_empty() {
                if account.code_hash == KECCAK_EMPTY {
                    account.code_hash = code.hash_slow();
                }
                self.db.contracts.entry(account.code_hash).or_insert_with(|| code.clone());
            }
        }
        if account.code_hash == B256::ZERO {
            account.code_hash = KECCAK_EMPTY;
        }
    }

    /// Modifies a storage slot of an account in the trie DB.
    ///
    /// # Takes
    /// - `address`: The address of the account.
    /// - `index`: The index of the storage slot.
    /// - `value`: The new value of the storage slot.
    ///
    /// # Returns
    /// - `Ok(())` if the storage slot was successfully modified.
    /// - `Err(_)` if the storage slot could not be modified.
    pub(crate) fn change_storage(
        &mut self,
        address: Address,
        index: U256,
        value: U256,
    ) -> Result<()> {
        let storage_root = self
            .storage_roots
            .get_mut(&address)
            .ok_or(anyhow!("Storage root not found for account: {address}"))?;
        let hashed_slot_key = keccak256(index.to_be_bytes::<32>().as_slice());

        let mut rlp_buf = Vec::with_capacity(value.length());
        value.encode(&mut rlp_buf);

        if let Ok(storage_slot_rlp) =
            storage_root.open(&Nibbles::unpack(hashed_slot_key), self.preimage_fetcher)
        {
            // If the storage slot already exists, update it.
            *storage_slot_rlp = rlp_buf.into();
        } else {
            // If the storage slot does not exist, insert it.
            storage_root.insert(
                &Nibbles::unpack(hashed_slot_key),
                rlp_buf.into(),
                self.preimage_fetcher,
            )?;
        }

        Ok(())
    }
}

impl<PF, CHF> DatabaseCommit for TrieCacheDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(B256) -> Result<Bytes> + Copy,
{
    fn commit(&mut self, updated_accounts: HashMap<Address, Account>) {
        let preimage_fetcher = self.preimage_fetcher;
        for (address, account) in updated_accounts {
            let account_path = Nibbles::unpack(keccak256(address.as_slice()));
            let mut trie_account = TrieAccount {
                balance: account.info.balance,
                nonce: account.info.nonce,
                code_hash: account.info.code_hash,
                ..Default::default()
            };

            // Update the account's storage root
            for (index, value) in account.storage {
                self.change_storage(address, index, value.present_value)
                    .expect("Failed to update account storage");
            }
            let acc_storage_root =
                self.storage_roots.get_mut(&address).expect("Storage root not found for account");
            acc_storage_root.blind();
            if let TrieNode::Blinded { commitment } = acc_storage_root {
                trie_account.storage_root = *commitment;
            } else {
                panic!("Storage root was not blinded successfully");
            }

            // RLP encode the account.
            let mut account_buf = Vec::with_capacity(trie_account.length());
            trie_account.encode(&mut account_buf);

            if let Ok(account_rlp_ref) = self.root_node.open(&account_path, preimage_fetcher) {
                // Update the existing account in the trie.
                *account_rlp_ref = account_buf.into();
            } else {
                // Insert the new account into the trie.
                self.root_node
                    .insert(&account_path, account_buf.into(), preimage_fetcher)
                    .expect("Failed to insert account into trie");
            }
        }

        // Update the root hash of the trie.
        self.state_root().expect("Failed to update state root");
    }
}

impl<PF, CHF> Database for TrieCacheDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(B256) -> Result<Bytes> + Copy,
{
    type Error = anyhow::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let basic = match self.db.accounts.entry(address) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(_) => {
                let account = self.load_account_from_trie(address)?;
                if let Some(ref code) = account.info.code {
                    self.db.contracts.insert(account.info.code_hash, code.clone());
                }
                self.db.accounts.insert(address, account);
                self.db
                    .accounts
                    .get_mut(&address)
                    .ok_or(anyhow!("Account not found in cache: {address}"))?
            }
        };
        Ok(basic.info())
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        match self.db.contracts.entry(code_hash) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(_) => anyhow::bail!("Code hash not found in cache: {code_hash}"),
        }
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        match self.db.accounts.entry(address) {
            Entry::Occupied(mut acc_entry) => {
                let acc_entry = acc_entry.get_mut();
                match acc_entry.storage.entry(index) {
                    Entry::Occupied(entry) => Ok(*entry.get()),
                    Entry::Vacant(_) => {
                        if matches!(
                            acc_entry.account_state,
                            AccountState::StorageCleared | AccountState::NotExisting
                        ) {
                            Ok(U256::ZERO)
                        } else {
                            let fetcher = self.preimage_fetcher;
                            let storage_root = self
                                .storage_roots
                                .get_mut(&address)
                                .ok_or(anyhow!("Storage root not found for account {address}"))?;

                            let hashed_slot_key = keccak256(index.to_be_bytes::<32>().as_slice());
                            let slot_value =
                                storage_root.open(&Nibbles::unpack(hashed_slot_key), fetcher)?;

                            let int_slot = U256::decode(&mut slot_value.as_ref())
                                .map_err(|e| anyhow!("Failed to decode storage slot value: {e}"))?;

                            self.db
                                .accounts
                                .get_mut(&address)
                                .ok_or(anyhow!("Account not found in cache: {address}"))?
                                .storage
                                .insert(index, int_slot);
                            Ok(int_slot)
                        }
                    }
                }
            }
            Entry::Vacant(_) => {
                // acc needs to be loaded for us to access slots.
                let info = self.basic(address)?;
                let (account, value) = if info.is_some() {
                    let value = self.storage(address, index)?;
                    let mut account: DbAccount = info.into();
                    account.storage.insert(index, value);
                    (account, value)
                } else {
                    (info.into(), U256::ZERO)
                };
                self.db.accounts.insert(address, account);
                Ok(value)
            }
        }
    }

    fn block_hash(&mut self, _: U256) -> Result<B256, Self::Error> {
        // match self.db.block_hashes.entry(number) {
        //     Entry::Occupied(entry) => Ok(*entry.get()),
        //     Entry::Vacant(_) => anyhow::bail!("Block hash for number not found"),
        // }
        unimplemented!("Block hash not implemented; Need to unroll the starting block hash for this operation.")
    }
}
