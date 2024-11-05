//! Error types for sources.

use super::{PipelineError, PipelineErrorKind};
use alloc::string::{String, ToString};

/// Blob Decoding Error
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

impl From<BlobProviderError> for PipelineErrorKind {
    fn from(val: BlobProviderError) -> Self {
        match val {
            BlobProviderError::SidecarLengthMismatch(_, _) => {
                PipelineError::Provider(val.to_string()).crit()
            }
            BlobProviderError::SlotDerivation => PipelineError::Provider(val.to_string()).crit(),
            BlobProviderError::BlobDecoding(_) => PipelineError::Provider(val.to_string()).crit(),
            BlobProviderError::Backend(_) => PipelineError::Provider(val.to_string()).temp(),
        }
    }
}

impl From<BlobDecodingError> for BlobProviderError {
    fn from(err: BlobDecodingError) -> Self {
        Self::BlobDecoding(err)
    }
}

impl core::error::Error for BlobProviderError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::BlobDecoding(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::error::Error;

    #[test]
    fn test_blob_decoding_error_source() {
        let err: BlobProviderError = BlobDecodingError::InvalidFieldElement.into();
        assert!(err.source().is_some());
    }

    #[test]
    fn test_from_blob_provider_error() {
        let err: PipelineErrorKind = BlobProviderError::SlotDerivation.into();
        assert!(matches!(err, PipelineErrorKind::Critical(_)));

        let err: PipelineErrorKind = BlobProviderError::SidecarLengthMismatch(1, 2).into();
        assert!(matches!(err, PipelineErrorKind::Critical(_)));

        let err: PipelineErrorKind =
            BlobProviderError::BlobDecoding(BlobDecodingError::InvalidFieldElement).into();
        assert!(matches!(err, PipelineErrorKind::Critical(_)));
    }
}
