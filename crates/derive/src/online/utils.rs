//! Contains utilities for online providers.

use crate::types::{Blob, BlobSidecar, IndexedBlobHash};
use alloc::vec::Vec;

/// Constructs a list of [Blob]s from [BlobSidecar]s and the specified [IndexedBlobHash]es.
pub(crate) fn blobs_from_sidecars(
    sidecars: &[BlobSidecar],
    hashes: &[IndexedBlobHash],
) -> anyhow::Result<Vec<Blob>> {
    if sidecars.len() != hashes.len() {
        return Err(anyhow::anyhow!(
            "blob sidecars and hashes length mismatch, {} != {}",
            sidecars.len(),
            hashes.len()
        ));
    }

    let mut blobs = Vec::with_capacity(sidecars.len());
    for (i, sidecar) in sidecars.iter().enumerate() {
        let hash = hashes.get(i).ok_or(anyhow::anyhow!("failed to get blob hash"))?;
        if sidecar.index as usize != hash.index {
            return Err(anyhow::anyhow!(
                "invalid sidecar ordering, blob hash index {} does not match sidecar index {}",
                hash.index,
                sidecar.index
            ));
        }

        // Ensure the blob's kzg commitment hashes to the expected value.
        if sidecar.to_kzg_versioned_hash() != hash.hash {
            return Err(anyhow::anyhow!(
                "expected hash {} for blob at index {} but got {:#?}",
                hash.hash,
                hash.index,
                sidecar.to_kzg_versioned_hash()
            ));
        }

        // Confirm blob data is valid by verifying its proof against the commitment
        match sidecar.verify_blob_kzg_proof() {
            Ok(true) => (),
            Ok(false) => {
                return Err(anyhow::anyhow!("blob at index {} failed verification", i));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("blob at index {} failed verification: {}", i, e));
            }
        }

        blobs.push(sidecar.blob);
    }
    Ok(blobs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::ToString, vec};

    #[test]
    fn test_blobs_from_sidecars_length_mismatch() {
        let sidecars = vec![BlobSidecar::default()];
        let hashes = vec![IndexedBlobHash::default(), IndexedBlobHash::default()];
        let err = blobs_from_sidecars(&sidecars, &hashes).unwrap_err();
        assert_eq!(err.to_string(), "blob sidecars and hashes length mismatch, 1 != 2");
    }
}
