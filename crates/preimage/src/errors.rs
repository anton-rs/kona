//! Errors for the `kona-preimage` crate.

use alloc::string::String;
use kona_common::errors::IOError;
use noerror::Error;

/// A [PreimageOracleError] is an enum that differentiates pipe-related errors from other errors
/// in the [PreimageOracleServer] and [HintReaderServer] implementations.
///
/// [PreimageOracleServer]: crate::PreimageOracleServer
/// [HintReaderServer]: crate::HintReaderServer
#[derive(Error, Debug)]
pub enum PreimageOracleError {
    /// The pipe has been broken.
    #[error(transparent)]
    IOError(#[from] IOError),
    /// The preimage key is invalid.
    #[error(transparent)]
    InvalidPreimageKey(#[from] InvalidPreimageKeyType),
    /// Key not found.
    #[error("Key not found.")]
    KeyNotFound,
    /// Buffer length mismatch.
    #[error("Buffer length mismatch. Expected {0}, got {1}.")]
    BufferLengthMismatch(usize, usize),
    /// Other errors.
    #[error("Error in preimage server: {0}")]
    Other(String),
}

/// A [Result] type for the [PreimageOracleError] enum.
pub type PreimageOracleResult<T> = Result<T, PreimageOracleError>;

/// Invalid [PreimageKeyType] error.
///
/// [PreimageKeyType]: crate::key::PreimageKeyType
#[derive(Error, Debug)]
#[error("Invalid preimage key type")]
pub struct InvalidPreimageKeyType;
