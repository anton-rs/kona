//! Contains utilities for online providers.

use crate::types::{Blob, BlobSidecar, IndexedBlobHash};
use alloc::vec::Vec;
use alloy_primitives::B256;

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
                "expected hash {} for blob at index {} but got {}",
                hash.hash,
                hash.index,
                B256::from(sidecar.to_kzg_versioned_hash())
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
    use crate::types::APIGetBlobSidecarsResponse;
    use alloc::{string::ToString, vec};
    use alloy_primitives::{b256, FixedBytes};

    #[test]
    fn test_blobs_from_sidecars_length_mismatch() {
        let sidecars = vec![BlobSidecar::default()];
        let hashes = vec![IndexedBlobHash::default(), IndexedBlobHash::default()];
        let err = blobs_from_sidecars(&sidecars, &hashes).unwrap_err();
        assert_eq!(err.to_string(), "blob sidecars and hashes length mismatch, 1 != 2");
    }

    #[test]
    fn test_blobs_from_sidecars_invalid_ordering() {
        let sidecars = vec![BlobSidecar::default()];
        let hashes = vec![IndexedBlobHash { index: 1, ..Default::default() }];
        let err = blobs_from_sidecars(&sidecars, &hashes).unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid sidecar ordering, blob hash index 1 does not match sidecar index 0"
        );
    }

    #[test]
    fn test_blobs_from_sidecars_invalid_hash() {
        let sidecars = vec![BlobSidecar::default()];
        let hashes =
            vec![IndexedBlobHash { hash: FixedBytes::from([1; 32]), ..Default::default() }];
        let err = blobs_from_sidecars(&sidecars, &hashes).unwrap_err();
        assert_eq!(
            err.to_string(),
            "expected hash 0x0101010101010101010101010101010101010101010101010101010101010101 for blob at index 0 but got 0x01b0761f87b081d5cf10757ccc89f12be355c70e2e29df288b65b30710dcbcd1"
        );
    }

    #[test]
    fn test_blobs_from_sidecars_failed_verification() {
        let sidecars = vec![BlobSidecar::default()];
        let hashes = vec![IndexedBlobHash {
            hash: b256!("01b0761f87b081d5cf10757ccc89f12be355c70e2e29df288b65b30710dcbcd1"),
            ..Default::default()
        }];
        let err = blobs_from_sidecars(&sidecars, &hashes).unwrap_err();
        assert_eq!(err.to_string(), "blob at index 0 failed verification");
    }

    #[test]
    fn test_blobs_from_sidecars_succeeds() {
        // Read in the test data
        let json_bytes = include_bytes!("testdata/eth_v1_beacon_sidecars_goerli.json");
        let sidecars: APIGetBlobSidecarsResponse = serde_json::from_slice(json_bytes).unwrap();
        let hashes = vec![
            IndexedBlobHash {
                index: 0,
                hash: b256!("011075cbb20f3235b3179a5dff22689c410cd091692180f4b6a12be77ea0f586"),
            },
            IndexedBlobHash {
                index: 1,
                hash: b256!("010a9e10aab79bab62e10a5b83c164a91451b6ef56d31ac95a9514ffe6d6b4e6"),
            },
            IndexedBlobHash {
                index: 2,
                hash: b256!("016122c8e41c69917b688240707d107aa6d2a480343e4e323e564241769a6b4a"),
            },
            IndexedBlobHash {
                index: 3,
                hash: b256!("01df1f9ae707f5847513c9c430b683182079edf2b1f94ee12e4daae7f3c8c309"),
            },
            IndexedBlobHash {
                index: 4,
                hash: b256!("01e5ee2f6cbbafb3c03f05f340e795fe5b5a8edbcc9ac3fc7bd3d1940b99ef3c"),
            },
        ];
        let blob_sidecars = sidecars.data.into_iter().map(|s| s.inner).collect::<Vec<_>>();
        let blobs = blobs_from_sidecars(&blob_sidecars, &hashes).unwrap();
        assert_eq!(blobs.len(), 5);
        for (i, blob) in blobs.iter().enumerate() {
            assert_eq!(blob.len(), 131072, "blob {} has incorrect length", i);
        }
    }
}
