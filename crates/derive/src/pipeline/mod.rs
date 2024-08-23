//! Module containing the derivation pipeline.

/// Re-export trait arguments.
pub use crate::traits::{
    ChainProvider, DataAvailabilityProvider, L2ChainProvider, NextAttributes, OriginAdvancer,
    OriginProvider, Pipeline, ResetProvider, ResettableStage, StepResult,
};

/// Re-export stage types that are needed as inputs.
pub use crate::stages::AttributesBuilder;

/// Re-export kona primitive types.
pub use kona_primitives::{BlockInfo, RollupConfig};

/// Re-export error types.
pub use crate::errors::{StageError, StageResult};

mod builder;
pub use builder::PipelineBuilder;

mod core;
pub use core::DerivationPipeline;
