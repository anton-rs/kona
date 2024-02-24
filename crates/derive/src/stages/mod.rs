//! This module contains each stage of the derivation pipeline, and offers a high-level API to functionally
//! apply each stage's output as an input to the next stage, until finally arriving at the produced execution
//! payloads.
//!
//! **Stages:**
//! 1. L1 Traversal
//! 2. L1 Retrieval
//! 3. Frame Queue
//! 4. Channel Bank
//! 5. Channel Reader (Batch Decoding)
//! 6. Batch Queue
//! 7. Payload Attributes Derivation
//! 8. Engine Queue

pub(crate) mod batch_queue;
pub(crate) mod channel_bank;
pub(crate) mod channel_reader;
pub(crate) mod engine_queue;
pub(crate) mod frame_queue;
pub(crate) mod l1_retrieval;
pub(crate) mod l1_traversal;
pub(crate) mod payload_derivation;
