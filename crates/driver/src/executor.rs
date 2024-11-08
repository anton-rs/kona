//! An abstraction for the driver's block executor.

use alloc::string::ToString;
use alloy_consensus::{Header, Sealed};
use alloy_primitives::B256;
use async_trait::async_trait;
use core::{
    error::Error,
    fmt::{Debug, Display},
};
use op_alloy_rpc_types_engine::OpPayloadAttributes;

/// Executor
///
/// Abstracts block execution by the driver.
#[async_trait]
pub trait Executor {
    /// The error type for the Executor.
    type Error: Error + Debug + Display + ToString;

    /// Execute the gicen [OpPayloadAttributes].
    fn execute_payload(&mut self, attributes: OpPayloadAttributes) -> Result<&Header, Self::Error>;

    /// Computes the output root.
    /// Expected to be called after the payload has been executed.
    fn compute_output_root(&mut self) -> Result<B256, Self::Error>;
}

/// Constructs the Executor.
///
/// This trait abstracts the construction of the Executor.
pub trait ExecutorConstructor<E>
where
    E: Executor,
{
    /// Construct the Executor.
    fn new_executor(&self, header: Sealed<Header>) -> E;
}
