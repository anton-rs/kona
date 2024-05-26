//! This module contains the [KeyValueStore] trait and concrete implementations of it.

use alloy_primitives::B256;

mod mem;
pub use mem::MemoryKeyValueStore;

/// Describes the interface of a simple, synchronous key-value store.
pub trait KeyValueStore {
    /// Get the value associated with the given key.
    fn get(&self, key: B256) -> Option<&Vec<u8>>;

    /// Set the value associated with the given key.
    fn set(&mut self, key: B256, value: Vec<u8>);
}
