//! Module containing the derivation pipeline.

/// Re-export trait arguments.
pub use crate::traits::{NextAttributes, OriginAdvancer, Pipeline, ResetProvider, ResettableStage};

/// Re-export commonly used types.
pub use crate::types::{StageError, StageResult};

mod builder;
pub use builder::PipelineBuilder;

mod core;
pub use core::DerivationPipeline;
