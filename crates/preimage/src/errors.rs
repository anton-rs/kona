//! Errors for the `kona-preimage` crate.

use alloc::string::String;
use kona_common::errors::IOError;

/// A [PreimageOracleError] is an enum that differentiates pipe-related errors from other errors
/// in the [PreimageOracleServer] and [HintReaderServer] implementations.
///
/// [PreimageOracleServer]: crate::PreimageOracleServer
/// [HintReaderServer]: crate::HintReaderServer
#[derive(derive_more::Display, Debug)]
pub enum PreimageOracleError {
    /// The pipe has been broken.
    #[display("{_0}")]
    IOError(IOError),
    /// The preimage key is invalid.
    #[display("{_0}")]
    InvalidPreimageKey(InvalidPreimageKeyType),
    /// Key not found.
    #[display("Key not found.")]
    KeyNotFound,
    /// Buffer length mismatch.
    #[display("Buffer length mismatch. Expected {_0}, got {_1}.")]
    BufferLengthMismatch(usize, usize),
    /// Other errors.
    #[display("Error in preimage server: {_0}")]
    Other(String),
}

impl From<IOError> for PreimageOracleError {
    fn from(err: IOError) -> Self {
        Self::IOError(err)
    }
}

impl From<InvalidPreimageKeyType> for PreimageOracleError {
    fn from(err: InvalidPreimageKeyType) -> Self {
        Self::InvalidPreimageKey(err)
    }
}

impl core::error::Error for PreimageOracleError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::IOError(err) => Some(err),
            _ => None,
        }
    }
}

/// A [Result] type for the [PreimageOracleError] enum.
pub type PreimageOracleResult<T> = Result<T, PreimageOracleError>;

/// Invalid [PreimageKeyType] error.
///
/// [PreimageKeyType]: crate::key::PreimageKeyType
#[derive(derive_more::Display, Debug)]
#[display("Invalid preimage key type")]
pub struct InvalidPreimageKeyType;

impl core::error::Error for InvalidPreimageKeyType {}
