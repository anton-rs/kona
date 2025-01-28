//! The [EnggineController] trait.

use alloc::boxed::Box;
use alloc::string::ToString;
use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadEnvelopeV2, ForkChoiceUpdateResult, ForkchoiceState,
    PayloadId, PayloadStatus,
};
use async_trait::async_trait;
use core::{
    error::Error,
    fmt::{Debug, Display},
};
use op_alloy_rpc_types_engine::{
    OpExecutionPayloadEnvelopeV3, OpExecutionPayloadEnvelopeV4, OpPayloadAttributes,
};

/// The [EngineController] trait defines a minimal asynchronous interface for interacting with the
/// Engine API on behalf of a rollup node.
///
/// Implementations of this trait are consumed by the [Driver] to execute blocks and update the
/// forkchoice of the execution layer.
#[async_trait]
pub trait EngineController {
    /// The error type for the Executor.
    type Error: Error + Debug + Display + ToString;

    /// Sends a forkchoice update to the execution layer, optionally building a new block in the process.
    async fn forkchoice_updated(
        &mut self,
        forkchoice_state: ForkchoiceState,
        payload_attributes: Option<OpPayloadAttributes>,
    ) -> ForkChoiceUpdateResult;

    /// Retrieves an [OpExecutionPayloadEnvelope] from the execution layer by [PayloadId].
    async fn get_payload(
        &mut self,
        payload_id: PayloadId,
    ) -> Result<OpExecutionPayloadEnvelope, Self::Error>;

    /// Execute the given [OpPayloadAttributes] and return either the sealed block header or an error.
    ///
    /// TODO: Enumerate payload envelope kinds.
    async fn new_payload(
        &mut self,
        attributes: ExecutionPayload,
    ) -> Result<PayloadStatus, Self::Error>;
}

/// This structure maps for the return value of `engine_getPayload` of OP Stack execution layers, for all supported
/// versions of the protocol.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum OpExecutionPayloadEnvelope {
    /// Version 2 of the execution payload envelope.
    V2(ExecutionPayloadEnvelopeV2),
    /// Version 3 of the execution payload envelope.
    V3(OpExecutionPayloadEnvelopeV3),
    /// Version 4 of the execution payload envelope.
    V4(OpExecutionPayloadEnvelopeV4),
}

// Deserializes untagged ExecutionPayload by trying each variant in falling order
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for OpExecutionPayloadEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum ExecutionPayloadDesc {
            V4(OpExecutionPayloadEnvelopeV4),
            V3(OpExecutionPayloadEnvelopeV3),
            V2(ExecutionPayloadEnvelopeV2),
        }
        match ExecutionPayloadDesc::deserialize(deserializer)? {
            ExecutionPayloadDesc::V4(payload) => Ok(Self::V4(payload)),
            ExecutionPayloadDesc::V3(payload) => Ok(Self::V3(payload)),
            ExecutionPayloadDesc::V2(payload) => Ok(Self::V2(payload)),
        }
    }
}
