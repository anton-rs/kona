//! Contains a concrete implementation of the [KeyValueStore] trait that stores data in memory.

use super::KeyValueStore;
use alloy_primitives::B256;
use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};

/// A simple, synchronous key-value store that stores data in memory. This is useful for testing and
/// development purposes.
#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct MemoryKeyValueStore {
    store: HashMap<B256, Vec<u8>>,
    json_path: Option<std::path::PathBuf>,
}

impl MemoryKeyValueStore {
    /// Create a new [MemoryKeyValueStore] with an empty store.
    pub fn new(json_path: Option<PathBuf>) -> Self {
        Self { store: HashMap::new(), json_path }
    }
}

impl KeyValueStore for MemoryKeyValueStore {
    fn get(&self, key: B256) -> Option<Vec<u8>> {
        self.store.get(&key).cloned()
    }

    fn set(&mut self, key: B256, value: Vec<u8>) {
        self.store.insert(key, value);
    }

    // TODO(ethan): move this out if it's not needed.
    fn export(&self) -> HashMap<B256, Vec<u8>> {
        self.store.clone()
    }

    fn export_json(&self) {
        if let Some(path) = &self.json_path {
            let store = self.export();
            let json = serde_json::to_string(&store).expect("Failed to serialize to JSON");
            let mut file = File::create(path).expect("Failed to create file");
            file.write_all(json.as_bytes()).expect("Failed to write to file");
        }
    }
}
