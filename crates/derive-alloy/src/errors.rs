//! Errors for the alloy-backed derivation providers.

use alloy_transport::RpcError;
use alloy_transport::TransportErrorKind;
use derive_more::{Display, Error};
use kona_derive::errors::PipelineError;
use kona_derive::errors::PipelineErrorKind;
use op_alloy_protocol::FromBlockError;
use op_alloy_protocol::OpBlockConversionError;

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

impl Into<PipelineErrorKind> for AlloyProviderError {
    fn into(self) -> PipelineErrorKind {
        match self {
            Self::Rlp(e) => PipelineError::Provider(e.to_string()).crit(),
            Self::BlockInfo(e) => PipelineError::Provider(e.to_string()).crit(),
            Self::OpBlockConversion(e) => PipelineError::Provider(e.to_string()).crit(),
            Self::Rpc(e) => PipelineError::Provider(e.to_string()).temp(),
        }
    }
}
