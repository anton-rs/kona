//! Indexed Blob Hash.

use alloy_primitives::B256;

/// A Blob hash
#[derive(Default, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IndexedBlobHash {
    /// The index of the blob
    pub index: usize,
    /// The hash of the blob
    pub hash: B256,
}

impl PartialEq for IndexedBlobHash {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.hash == other.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexed_blob_hash() {
        let hash = B256::from([1; 32]);
        let indexed_blob_hash = IndexedBlobHash { index: 1, hash };

        assert_eq!(indexed_blob_hash.index, 1);
        assert_eq!(indexed_blob_hash.hash, hash);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_indexed_blob_hash_serde_roundtrip() {
        let hash = B256::from([1; 32]);
        let indexed_blob_hash = IndexedBlobHash { index: 1, hash };

        let serialized = serde_json::to_string(&indexed_blob_hash).unwrap();
        let deserialized: IndexedBlobHash = serde_json::from_str(&serialized).unwrap();

        assert_eq!(indexed_blob_hash, deserialized);
    }
}
