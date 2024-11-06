//! Contains driver-related error types.

use derive_more::derive::Display;
use kona_derive::errors::PipelineErrorKind;
use op_alloy_protocol::FromBlockError;

/// A [Result] type for the [DriverError].
pub type DriverResult<T, E> = Result<T, DriverError<E>>;

/// Driver error.
#[derive(Display, Debug)]
pub enum DriverError<E>
where
    E: core::error::Error,
{
    /// Pipeline error.
    #[display("Pipeline error: {_0}")]
    Pipeline(PipelineErrorKind),
    /// An error returned by the executor.
    #[display("Executor error: {_0}")]
    Executor(E),
    /// An error returned by the conversion from a block to an [op_alloy_protocol::L2BlockInfo].
    #[display("From block error: {_0}")]
    FromBlock(FromBlockError),
    /// Error decoding or encoding RLP.
    #[display("RLP error: {_0}")]
    Rlp(alloy_rlp::Error),
}

impl<E> core::error::Error for DriverError<E> where E: core::error::Error {}

impl<E> From<PipelineErrorKind> for DriverError<E>
where
    E: core::error::Error,
{
    fn from(val: PipelineErrorKind) -> Self {
        Self::Pipeline(val)
    }
}

impl<E> From<FromBlockError> for DriverError<E>
where
    E: core::error::Error,
{
    fn from(val: FromBlockError) -> Self {
        Self::FromBlock(val)
    }
}

impl<E> From<alloy_rlp::Error> for DriverError<E>
where
    E: core::error::Error,
{
    fn from(val: alloy_rlp::Error) -> Self {
        Self::Rlp(val)
    }
}
