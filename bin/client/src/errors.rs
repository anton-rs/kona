//! Error types for the client program.

use alloc::string::{String, ToString};
use derive_more::derive::Display;
use kona_derive::errors::{PipelineError, PipelineErrorKind};
use kona_mpt::OrderedListWalkerError;
use kona_preimage::errors::PreimageOracleError;
use op_alloy_protocol::{FromBlockError, OpBlockConversionError};

/// Error from an oracle-backed provider.
#[derive(Display, Debug)]
pub enum OracleProviderError {
    /// Requested block number is past the chain head.
    #[display("Block number ({_0}) past chain head ({_1})")]
    BlockNumberPastHead(u64, u64),
    /// Preimage oracle error.
    #[display("Preimage oracle error: {_0}")]
    Preimage(PreimageOracleError),
    /// List walker error.
    #[display("Trie walker error: {_0}")]
    TrieWalker(OrderedListWalkerError),
    /// BlockInfo error.
    #[display("From block error: {_0}")]
    BlockInfo(FromBlockError),
    /// Op Block conversion error.
    #[display("Op block conversion error: {_0}")]
    OpBlockConversion(OpBlockConversionError),
    /// Error decoding or encoding RLP.
    #[display("RLP error: {_0}")]
    Rlp(alloy_rlp::Error),
    /// Slice conversion error.
    #[display("Slice conversion error: {_0}")]
    SliceConversion(core::array::TryFromSliceError),
    /// Serde error.
    #[display("Serde error: {_0}")]
    Serde(serde_json::Error),
}

impl core::error::Error for OracleProviderError {}

impl From<OracleProviderError> for PipelineErrorKind {
    fn from(val: OracleProviderError) -> Self {
        match val {
            OracleProviderError::BlockNumberPastHead(_, _) => PipelineError::EndOfSource.crit(),
            _ => PipelineError::Provider(val.to_string()).crit(),
        }
    }
}

/// Error parsing a hint.
#[derive(Display, Debug)]
#[display("Hint parsing error: {_0}")]
pub struct HintParsingError(pub String);

impl core::error::Error for HintParsingError {}
