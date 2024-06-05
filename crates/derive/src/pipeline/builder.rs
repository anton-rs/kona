//! Contains the `PipelineBuilder` object that is used to build a `DerivationPipeline`.

use super::{DerivationPipeline, NextAttributes, OriginAdvancer, ResetProvider, ResettableStage};
use alloc::collections::VecDeque;
use core::fmt::Debug;
use kona_primitives::L2BlockInfo;

/// The PipelineBuilder constructs a [DerivationPipeline].
#[derive(Debug)]
pub struct PipelineBuilder<S, R>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    R: ResetProvider + Send,
{
    attributes: Option<S>,
    reset: Option<R>,
    start_cursor: Option<L2BlockInfo>,
}

impl<S, R> PipelineBuilder<S, R>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    R: ResetProvider + Send,
{
    /// Sets the attributes for the pipeline.
    pub fn attributes(mut self, attributes: S) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Sets the reset provider for the pipeline.
    pub fn reset(mut self, reset: R) -> Self {
        self.reset = Some(reset);
        self
    }

    /// Sets the start cursor for the pipeline.
    pub fn start_cursor(mut self, cursor: L2BlockInfo) -> Self {
        self.start_cursor = Some(cursor);
        self
    }

    /// Builds the pipeline.
    pub fn build(self) -> DerivationPipeline<S, R> {
        self.into()
    }
}

impl<S, R> From<PipelineBuilder<S, R>> for DerivationPipeline<S, R>
where
    S: NextAttributes + ResettableStage + OriginAdvancer + Debug + Send,
    R: ResetProvider + Send,
{
    fn from(builder: PipelineBuilder<S, R>) -> Self {
        let attributes = builder.attributes.expect("attributes must be set");
        let reset = builder.reset.expect("reset must be set");
        let start_cursor = builder.start_cursor.expect("start_cursor must be set");

        DerivationPipeline {
            attributes,
            reset,
            prepared: VecDeque::new(),
            needs_reset: false,
            cursor: start_cursor,
        }
    }
}
