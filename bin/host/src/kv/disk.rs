//! Contains a concrete implementation of the [KeyValueStore] trait that stores data on disk.
//!
//! Data is stored in a directory, with a separate file for each key. The key is the filename, and
//! the value is the raw contents of the file.

use super::KeyValueStore;
use alloy_primitives::{hex, B256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

/// A simple, synchronous key-value store that stores data on disk.
#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct DiskKeyValueStore {
    data_directory: PathBuf,
    keys: Vec<alloy_primitives::B256>,
    json_path: Option<PathBuf>,
}

impl DiskKeyValueStore {
    /// Create a new [DiskKeyValueStore] with the given data directory.
    pub fn new(data_directory: PathBuf, json_path: Option<PathBuf>) -> Self {
        Self { data_directory, keys: Vec::new(), json_path }
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
        self.keys.push(key);
    }

    fn export(&self) -> HashMap<alloy_primitives::B256, Vec<u8>> {
        let mut store = HashMap::new();
        let entries = fs::read_dir(&self.data_directory).unwrap();
        for entry in entries {
            let path = entry.unwrap().path();
            let file_name = path
                .file_name()
                .expect("Wrong file name")
                .to_str()
                .expect("Failed to convert to string")
                .strip_suffix(".bin")
                .expect("Failed to strip prefix");
            let key_bytes = hex::decode(file_name).unwrap();
            let key = B256::from_slice(&key_bytes);
            if let Some(value) = self.get(key) {
                store.insert(key, value);
            } else {
                panic!("the key was cached, but the value was not found on disk");
            }
        }
        store
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
