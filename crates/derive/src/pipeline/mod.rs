//! Module containing the derivation pipeline.

/// Re-export trait arguments.
pub use crate::traits::{
    ChainProvider, DataAvailabilityProvider, L2ChainProvider, NextAttributes, OriginAdvancer,
    Pipeline, PreviousStage, ResetProvider, ResettableStage,
};

/// Re-export stage types that are needed as inputs.
pub use crate::stages::AttributesBuilder;

/// Re-export commonly used types.
pub use crate::types::{RollupConfig, StageError, StageResult};

mod builder;
pub use builder::PipelineBuilder;

mod core;
pub use core::DerivationPipeline;
