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

mod l1_traversal;
pub use l1_traversal::L1Traversal;

mod batch_queue;
mod channel_bank;
mod channel_reader;
mod engine_queue;
mod frame_queue;
mod l1_retrieval;
mod payload_derivation;
