//! Contains a concrete implementation of the [KeyValueStore] trait that stores data in memory.

use super::KeyValueStore;
use alloy_primitives::B256;
use anyhow::Result;
use std::collections::HashMap;

/// A simple, synchronous key-value store that stores data in memory. This is useful for testing and
/// development purposes.
#[derive(
    Default, Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct MemoryKeyValueStore {
    pub store: HashMap<[u8; 32], Vec<u8>>,
}

impl MemoryKeyValueStore {
    /// Create a new [MemoryKeyValueStore] with an empty store.
    pub fn new() -> Self {
        Self { store: HashMap::new() }
    }
}

impl KeyValueStore for MemoryKeyValueStore {
    fn get(&self, key: B256) -> Option<Vec<u8>> {
        self.store.get(&key.0).cloned()
    }

    fn set(&mut self, key: B256, value: Vec<u8>) -> Result<()> {
        self.store.insert(key.0, value);
        Ok(())
    }

    fn to_memory_store(&self) -> MemoryKeyValueStore {
        self.clone()
    }
}
