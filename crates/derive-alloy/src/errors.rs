//! Errors for the alloy-backed derivation providers.

use alloy_transport::{RpcError, TransportErrorKind};
use derive_more::{Display, Error};
use kona_derive::errors::{PipelineError, PipelineErrorKind};
use op_alloy_protocol::{FromBlockError, OpBlockConversionError};

/// Error from an alloy-backed provider.
#[derive(Error, Display, Debug)]
pub enum AlloyProviderError {
    /// An [RpcError] occurred.
    #[display("RPC Error: {_0}")]
    Rpc(RpcError<TransportErrorKind>),
    /// A [alloy_rlp::Error] occurred.
    #[display("RLP Error: {_0}")]
    Rlp(alloy_rlp::Error),
    /// BlockInfo error.
    #[display("From block error: {_0}")]
    BlockInfo(FromBlockError),
    /// Op Block conversion error.
    #[display("Op block conversion error: {_0}")]
    OpBlockConversion(OpBlockConversionError),
}

impl From<AlloyProviderError> for PipelineErrorKind {
    fn from(val: AlloyProviderError) -> Self {
        match val {
            AlloyProviderError::Rlp(e) => PipelineError::Provider(e.to_string()).crit(),
            AlloyProviderError::BlockInfo(e) => PipelineError::Provider(e.to_string()).crit(),
            AlloyProviderError::OpBlockConversion(e) => {
                PipelineError::Provider(e.to_string()).crit()
            }
            AlloyProviderError::Rpc(e) => PipelineError::Provider(e.to_string()).temp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_alloy_provider_error() {
        let err: PipelineErrorKind = AlloyProviderError::Rlp(alloy_rlp::Error::Overflow).into();
        assert!(matches!(err, PipelineErrorKind::Critical(_)));

        let err: PipelineErrorKind =
            AlloyProviderError::BlockInfo(FromBlockError::InvalidGenesisHash).into();
        assert!(matches!(err, PipelineErrorKind::Critical(_)));

        let err: PipelineErrorKind = AlloyProviderError::OpBlockConversion(
            OpBlockConversionError::MissingSystemConfigGenesis,
        )
        .into();
        assert!(matches!(err, PipelineErrorKind::Critical(_)));

        let err: PipelineErrorKind = AlloyProviderError::Rpc(RpcError::NullResp).into();
        assert!(matches!(err, PipelineErrorKind::Temporary(_)));
    }
}
