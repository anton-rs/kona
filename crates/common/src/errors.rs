//! Errors for the `kona-common` crate.

/// An error that can occur when reading from or writing to a file descriptor.
#[derive(derive_more::Display, Debug, PartialEq, Eq)]
#[display("IO error (errno: {_0})")]
pub struct IOError(pub i32);

impl core::error::Error for IOError {}

/// A [Result] type for the [IOError].
pub type IOResult<T> = Result<T, IOError>;
