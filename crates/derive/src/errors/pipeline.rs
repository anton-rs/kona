//! This module contains derivation errors thrown within the pipeline.

use crate::errors::BuilderError;
use alloc::string::String;
use alloy_primitives::B256;
use op_alloy_genesis::system::SystemConfigUpdateError;
use op_alloy_protocol::{DepositError, SpanBatchError};

/// [crate::ensure] is a short-hand for bubbling up errors in the case of a condition not being met.
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err);
        }
    };
}

/// A top level filter for [PipelineError] that sorts by severity.
#[derive(derive_more::Display, Debug, PartialEq, Eq)]
pub enum PipelineErrorKind {
    /// A temporary error.
    #[display("Temporary error: {_0}")]
    Temporary(PipelineError),
    /// A critical error.
    #[display("Critical error: {_0}")]
    Critical(PipelineError),
    /// A reset error.
    #[display("Pipeline reset: {_0}")]
    Reset(ResetError),
}

impl From<ResetError> for PipelineErrorKind {
    fn from(err: ResetError) -> Self {
        Self::Reset(err)
    }
}

impl core::error::Error for PipelineErrorKind {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Temporary(err) => Some(err),
            Self::Critical(err) => Some(err),
            Self::Reset(err) => Some(err),
        }
    }
}

/// An error encountered during the processing.
#[derive(derive_more::Display, Debug, PartialEq, Eq)]
pub enum PipelineError {
    /// There is no data to read from the channel bank.
    #[display("EOF")]
    Eof,
    /// There is not enough data to complete the processing of the stage. If the operation is
    /// re-tried, more data will come in allowing the pipeline to progress, or eventually a
    /// [PipelineError::Eof] will be encountered.
    #[display("Not enough data")]
    NotEnoughData,
    /// No channels are available in the [ChannelProvider].
    ///
    /// [ChannelProvider]: crate::stages::ChannelProvider
    #[display("The channel provider is empty")]
    ChannelProviderEmpty,
    /// The channel has already been built by the [ChannelAssembler] stage.
    ///
    /// [ChannelAssembler]: crate::stages::ChannelAssembler
    #[display("Channel already built")]
    ChannelAlreadyBuilt,
    /// Failed to find channel in the [ChannelProvider].
    ///
    /// [ChannelProvider]: crate::stages::ChannelProvider
    #[display("Channel not found in channel provider")]
    ChannelNotFound,
    /// No channel returned by the [ChannelReader] stage.
    ///
    /// [ChannelReader]: crate::stages::ChannelReader
    #[display("The channel reader has no channel available")]
    ChannelReaderEmpty,
    /// The [BatchQueue] is empty.
    ///
    /// [BatchQueue]: crate::stages::BatchQueue
    #[display("The batch queue has no batches available")]
    BatchQueueEmpty,
    /// Missing L1 origin.
    #[display("Missing L1 origin from previous stage")]
    MissingOrigin,
    /// Missing data from [L1Retrieval].
    ///
    /// [L1Retrieval]: crate::stages::L1Retrieval
    #[display("L1 Retrieval missing data")]
    MissingL1Data,
    /// Invalid batch type passed.
    #[display("Invalid batch type passed to stage")]
    InvalidBatchType,
    /// Invalid batch validity variant.
    #[display("Invalid batch validity")]
    InvalidBatchValidity,
    /// [SystemConfig] update error.
    ///
    /// [SystemConfig]: op_alloy_genesis::SystemConfig
    #[display("Error updating system config: {_0}")]
    SystemConfigUpdate(SystemConfigUpdateError),
    /// Attributes builder error variant, with [BuilderError].
    #[display("Attributes builder error: {_0}")]
    AttributesBuilder(BuilderError),
    /// [PipelineEncodingError] variant.
    #[display("Decode error: {_0}")]
    BadEncoding(PipelineEncodingError),
    /// The data source can no longer provide any more data.
    #[display("Data source exhausted")]
    EndOfSource,
    /// Provider error variant.
    #[display("Blob provider error: {_0}")]
    Provider(String),
}

impl From<BuilderError> for PipelineError {
    fn from(err: BuilderError) -> Self {
        Self::AttributesBuilder(err)
    }
}

impl From<PipelineEncodingError> for PipelineError {
    fn from(err: PipelineEncodingError) -> Self {
        Self::BadEncoding(err)
    }
}

impl core::error::Error for PipelineError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::AttributesBuilder(err) => Some(err),
            Self::BadEncoding(err) => Some(err),
            _ => None,
        }
    }
}

impl PipelineError {
    /// Wrap [PipelineError] as a [PipelineErrorKind::Critical].
    pub const fn crit(self) -> PipelineErrorKind {
        PipelineErrorKind::Critical(self)
    }

    /// Wrap [PipelineError] as a [PipelineErrorKind::Temporary].
    pub const fn temp(self) -> PipelineErrorKind {
        PipelineErrorKind::Temporary(self)
    }
}

/// A reset error
#[derive(derive_more::Display, Clone, Debug, Eq, PartialEq)]
pub enum ResetError {
    /// The batch has a bad parent hash.
    /// The first argument is the expected parent hash, and the second argument is the actual
    /// parent hash.
    #[display("Bad parent hash: expected {_0}, got {_1}")]
    BadParentHash(B256, B256),
    /// The batch has a bad timestamp.
    /// The first argument is the expected timestamp, and the second argument is the actual
    /// timestamp.
    #[display("Bad timestamp: expected {_0}, got {_1}")]
    BadTimestamp(u64, u64),
    /// L1 origin mismatch.
    #[display("L1 origin mismatch. Expected {_0:?}, got {_1:?}")]
    L1OriginMismatch(u64, u64),
    /// The stage detected a block reorg.
    /// The first argument is the expected block hash.
    /// The second argument is the parent_hash of the next l1 origin block.
    #[display("L1 reorg detected: expected {_0}, got {_1}")]
    ReorgDetected(B256, B256),
    /// Attributes builder error variant, with [BuilderError].
    #[display("Attributes builder error: {_0}")]
    AttributesBuilder(BuilderError),
    /// A Holocene activation temporary error.
    #[display("Holocene activation reset")]
    HoloceneActivation,
}

impl From<BuilderError> for ResetError {
    fn from(err: BuilderError) -> Self {
        Self::AttributesBuilder(err)
    }
}

impl core::error::Error for ResetError {}

impl ResetError {
    /// Wrap [ResetError] as a [PipelineErrorKind::Reset].
    pub const fn reset(self) -> PipelineErrorKind {
        PipelineErrorKind::Reset(self)
    }
}

/// A decoding error.
#[derive(derive_more::Display, Debug, PartialEq, Eq)]
pub enum PipelineEncodingError {
    /// The buffer is empty.
    #[display("Empty buffer")]
    EmptyBuffer,
    /// Deposit decoding error.
    #[display("Error decoding deposit: {_0}")]
    DepositError(DepositError),
    /// Alloy RLP Encoding Error.
    #[display("RLP error: {_0}")]
    AlloyRlpError(alloy_rlp::Error),
    /// Span Batch Error.
    #[display("{_0}")]
    SpanBatchError(SpanBatchError),
}

impl core::error::Error for PipelineEncodingError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::DepositError(err) => Some(err),
            Self::SpanBatchError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<SpanBatchError> for PipelineEncodingError {
    fn from(err: SpanBatchError) -> Self {
        Self::SpanBatchError(err)
    }
}

impl From<DepositError> for PipelineEncodingError {
    fn from(err: DepositError) -> Self {
        Self::DepositError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::error::Error;

    #[test]
    fn test_pipeline_error_kind_source() {
        let err = PipelineErrorKind::Temporary(PipelineError::Eof);
        assert!(err.source().is_some());

        let err = PipelineErrorKind::Critical(PipelineError::Eof);
        assert!(err.source().is_some());

        let err = PipelineErrorKind::Reset(ResetError::BadParentHash(
            Default::default(),
            Default::default(),
        ));
        assert!(err.source().is_some());
    }

    #[test]
    fn test_pipeline_error_source() {
        let err = PipelineError::AttributesBuilder(BuilderError::BlockMismatch(
            Default::default(),
            Default::default(),
        ));
        assert!(err.source().is_some());

        let encoding_err = PipelineEncodingError::EmptyBuffer;
        let err: PipelineError = encoding_err.into();
        assert!(err.source().is_some());

        let err = PipelineError::Eof;
        assert!(err.source().is_none());
    }

    #[test]
    fn test_pipeline_encoding_error_source() {
        let err = PipelineEncodingError::DepositError(DepositError::UnexpectedTopicsLen(0));
        assert!(err.source().is_some());

        let err = SpanBatchError::TooBigSpanBatchSize;
        let err: PipelineEncodingError = err.into();
        assert!(err.source().is_some());

        let err = PipelineEncodingError::EmptyBuffer;
        assert!(err.source().is_none());
    }

    #[test]
    fn test_reset_error_kinds() {
        let reset_errors = [
            ResetError::BadParentHash(Default::default(), Default::default()),
            ResetError::BadTimestamp(0, 0),
            ResetError::L1OriginMismatch(0, 0),
            ResetError::ReorgDetected(Default::default(), Default::default()),
            ResetError::AttributesBuilder(BuilderError::BlockMismatch(
                Default::default(),
                Default::default(),
            )),
            ResetError::HoloceneActivation,
        ];
        for error in reset_errors.into_iter() {
            let expected = PipelineErrorKind::Reset(error.clone());
            assert_eq!(error.reset(), expected);
        }
    }
}
