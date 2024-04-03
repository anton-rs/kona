//! This module contains each stage of the derivation pipeline, and offers a high-level API to
//! functionally apply each stage's output as an input to the next stage, until finally arriving at
//! the produced execution payloads.
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

mod l1_retrieval;
pub use l1_retrieval::L1Retrieval;

mod frame_queue;
pub use frame_queue::FrameQueue;

mod channel_bank;
pub use channel_bank::ChannelBank;

mod channel_reader;
pub use channel_reader::ChannelReader;

mod batch_queue;
pub use batch_queue::BatchQueue;

mod attributes_queue;
pub use attributes_queue::AttributesQueue;
