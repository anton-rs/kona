//! EIP4844 Blob Type

use anyhow::Result;

use alloy_primitives::{Bytes, FixedBytes, B256};

/// How many bytes are in a blob
pub(crate) const BYTES_PER_BLOB: usize = 131072;

/// A Blob serialized as 0x-prefixed hex string
pub type Blob = FixedBytes<BYTES_PER_BLOB>;

/// A Blob hash
#[derive(Default, Clone, Debug)]
pub struct IndexedBlobHash {
    /// The index of the blob
    pub index: usize,
    /// The hash of the blob
    pub hash: B256,
}

/// The Blob Data
#[derive(Default, Clone, Debug)]
pub struct BlobData {
    /// The blob data
    pub data: Option<Bytes>,
    /// The calldata
    pub calldata: Option<Bytes>,
}

impl BlobData {
    /// Fills in the pointers to the fetched blob bodies.
    /// There should be exactly one placeholder blobOrCalldata
    /// element for each blob, otherwise an error is returned.
    pub fn fill(&mut self, blobs: &[Blob], index: usize) -> Result<()> {
        // Do not fill if there is no calldata to fill
        if self.calldata.as_ref().map_or(false, |data| data.is_empty()) {
            return Ok(());
        }

        if index >= blobs.len() {
            return Err(anyhow::anyhow!("Insufficient blob count"));
        }

        if blobs[index].is_empty() {
            return Err(anyhow::anyhow!("Empty blob"));
        }

        self.data = Some(Bytes::from(blobs[index]));
        Ok(())
    }

    /// Returns if a blob is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_none() && self.calldata.is_none()
    }
}

impl BlobData {
    /// Turns the blob into its inner data.
    pub fn inner(&self) -> anyhow::Result<Bytes> {
        if let Some(data) = &self.calldata {
            return Ok(data.clone());
        }
        if let Some(data) = &self.data {
            return Ok(data.clone());
        }
        Err(anyhow::anyhow!("No data found"))
    }
}
