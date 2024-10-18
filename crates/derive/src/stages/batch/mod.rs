//! Contains stages pertaining to the processing of [Batch]es.
//!
//! Sitting after the [ChannelReader] stage, the [BatchStream] and [BatchQueue] stages are
//! responsible for validating and ordering the [Batch]es. The [BatchStream] stage is responsible
//! for streaming [SingleBatch]es from [SpanBatch]es, while the [BatchQueue] stage is responsible
//! for ordering the [Batch]es for the [AttributesQueue] stage.
//!
//! [Batch]: crate::batch::Batch
//! [SingleBatch]: crate::batch::SingleBatch
//! [SpanBatch]: crate::batch::SpanBatch
//! [ChannelReader]: crate::stages::channel::ChannelReader
//! [AttributesQueue]: crate::stages::attributes_queue::AttributesQueue

use crate::{batch::Batch, pipeline::PipelineResult};
use alloc::boxed::Box;
use async_trait::async_trait;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};

mod batch_stream;
pub use batch_stream::{BatchStream, BatchStreamProvider};

mod batch_queue;
pub use batch_queue::BatchQueue;

mod batch_validator;
pub use batch_validator::BatchValidator;

/// Provides [Batch]es for the [BatchQueue] and [BatchValidator] stages.
#[async_trait]
pub trait NextBatchProvider {
    /// Returns the next [Batch] in the [ChannelReader] stage, if the stage is not complete.
    /// This function can only be called once while the stage is in progress, and will return
    /// [`None`] on subsequent calls unless the stage is reset or complete. If the stage is
    /// complete and the batch has been consumed, an [PipelineError::Eof] error is returned.
    ///
    /// [ChannelReader]: crate::stages::ChannelReader
    /// [PipelineError::Eof]: crate::errors::PipelineError::Eof
    async fn next_batch(
        &mut self,
        parent: L2BlockInfo,
        l1_origins: &[BlockInfo],
    ) -> PipelineResult<Batch>;

    /// Returns the number of [SingleBatch]es that are currently buffered in the [BatchStream]
    /// from a [SpanBatch].
    ///
    /// [SpanBatch]: crate::batch::SpanBatch
    /// [SingleBatch]: crate::batch::SingleBatch
    fn span_buffer_size(&self) -> usize;

    /// Allows the stage to flush the buffer in the [crate::stages::BatchStream]
    /// if an invalid single batch is found. Pre-holocene hardfork, this will be a no-op.
    fn flush(&mut self);
}
