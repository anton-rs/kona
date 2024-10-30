//! Error types for derivation pipeline sources.

use alloc::string::String;

/// Blob Decuding Error
#[derive(derive_more::Display, Debug, PartialEq, Eq)]
pub enum BlobDecodingError {
    /// Invalid field element
    #[display("Invalid field element")]
    InvalidFieldElement,
    /// Invalid encoding version
    #[display("Invalid encoding version")]
    InvalidEncodingVersion,
    /// Invalid length
    #[display("Invalid length")]
    InvalidLength,
    /// Missing Data
    #[display("Missing data")]
    MissingData,
}

impl core::error::Error for BlobDecodingError {}

/// An error returned by the [BlobProviderError].
#[derive(derive_more::Display, Debug, PartialEq, Eq)]
pub enum BlobProviderError {
    /// The number of specified blob hashes did not match the number of returned sidecars.
    #[display("Blob sidecar length mismatch: expected {_0}, got {_1}")]
    SidecarLengthMismatch(usize, usize),
    /// Slot derivation error.
    #[display("Failed to derive slot")]
    SlotDerivation,
    /// Blob decoding error.
    #[display("Blob decoding error: {_0}")]
    BlobDecoding(BlobDecodingError),
    /// Error pertaining to the backend transport.
    #[display("{_0}")]
    Backend(String),
}

impl From<BlobDecodingError> for BlobProviderError {
    fn from(err: BlobDecodingError) -> Self {
        Self::BlobDecoding(err)
    }
}

impl core::error::Error for BlobProviderError {}

