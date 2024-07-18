//! Contains a concrete implementation of the [KeyValueStore] trait that splits between two separate
//! [KeyValueStore]s depending on [PreimageKeyType].

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use alloy_primitives::B256;
use kona_preimage::PreimageKeyType;

use super::KeyValueStore;

/// A split implementation of the [KeyValueStore] trait that splits between two separate
/// [KeyValueStore]s.
#[derive(Clone)]
pub struct SplitKeyValueStore<L, R>
where
    L: KeyValueStore,
    R: KeyValueStore,
{
    local_store: L,
    remote_store: R,
    json_path: Option<PathBuf>,
}

impl<L, R> SplitKeyValueStore<L, R>
where
    L: KeyValueStore,
    R: KeyValueStore,
{
    /// Create a new [SplitKeyValueStore] with the given left and right [KeyValueStore]s.
    pub fn new(local_store: L, remote_store: R, json_path: Option<PathBuf>) -> Self {
        Self { local_store, remote_store, json_path }
    }
}

impl<L, R> KeyValueStore for SplitKeyValueStore<L, R>
where
    L: KeyValueStore,
    R: KeyValueStore,
{
    fn get(&self, key: B256) -> Option<Vec<u8>> {
        match PreimageKeyType::try_from(key[0]).ok()? {
            PreimageKeyType::Local => self.local_store.get(key),
            _ => self.remote_store.get(key),
        }
    }

    fn set(&mut self, key: B256, value: Vec<u8>) {
        self.remote_store.set(key, value);
    }

    fn export(&self) -> std::collections::HashMap<B256, Vec<u8>> {
        let mut map = self.local_store.export();
        map.extend(self.remote_store.export());
        map
    }

    fn export_json(&self) {
        if let Some(path) = &self.json_path {
            let store = self.export();
            let json = serde_json::to_string(&store).expect("Failed to serialize to JSON");

            let dir_path = Path::new(&path).parent().expect("Failed to get parent path");
            if let Err(e) = fs::create_dir_all(dir_path) {
                panic!("Failed to create directories: {}", e);
            }

            let mut file = File::create(path).expect("Failed to create file");
            file.write_all(json.as_bytes()).expect("Failed to write to file");
        }
    }
}
