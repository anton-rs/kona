//! Module containing the derivation pipeline.

/// Re-export trait arguments.
pub use crate::traits::{
    AttributesQueueBuilder, ChainProvider, DataAvailabilityProvider, L2ChainProvider,
    NextAttributes, OriginAdvancer, OriginProvider, Pipeline, ResetProvider, ResettableStage,
    StepResult,
};

/// Re-export error types.
pub use crate::errors::{PipelineError, PipelineResult};

mod builder;
pub use builder::PipelineBuilder;

mod core;
pub use core::DerivationPipeline;
