//! Contains a concrete implementation of the [KeyValueStore] trait that stores data on disk.
//!
//! Data is stored in a directory, with a separate file for each key. The key is the filename, and
//! the value is the raw contents of the file.

use super::KeyValueStore;
use alloy_primitives::hex;
use std::{fs, path::PathBuf};

/// A simple, synchronous key-value store that stores data on disk.
#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct DiskKeyValueStore {
    data_directory: PathBuf,
}

impl DiskKeyValueStore {
    /// Create a new [DiskKeyValueStore] with the given data directory.
    pub fn new(data_directory: PathBuf) -> Self {
        Self { data_directory }
    }
}

impl KeyValueStore for DiskKeyValueStore {
    fn get(&self, key: alloy_primitives::B256) -> Option<Vec<u8>> {
        let path = self.data_directory.join(format!("{}.bin", hex::encode(key)));
        fs::create_dir_all(&self.data_directory).ok()?;
        fs::read(path).ok()
    }

    fn set(&mut self, key: alloy_primitives::B256, value: Vec<u8>) {
        let path = self.data_directory.join(format!("{}.bin", hex::encode(key)));
        fs::create_dir_all(&self.data_directory).expect("Failed to create directory");
        fs::write(path, value.as_slice()).expect("Failed to write data to disk");
    }
}
