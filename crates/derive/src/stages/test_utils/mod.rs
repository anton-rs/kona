//! Test utilities for the stages module primarily contains
//! mock implementations of the various stages for testing.

mod batch_queue;
pub use batch_queue::MockBatchQueueProvider;

mod attributes_queue;
pub use attributes_queue::{
    new_attributes_provider, MockAttributesBuilder, MockAttributesProvider,
};

mod frame_queue;
pub use frame_queue::MockFrameQueueProvider;

mod channel_bank;
pub use channel_bank::MockChannelBankProvider;

mod channel_reader;
pub use channel_reader::MockChannelReaderProvider;

mod tracing;
pub use tracing::{CollectingLayer, TraceStorage};

mod sys_config_fetcher;
pub use sys_config_fetcher::MockSystemConfigL2Fetcher;
