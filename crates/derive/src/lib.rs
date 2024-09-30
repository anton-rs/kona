#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(any(test, feature = "metrics")), no_std)]
#![cfg_attr(not(any(test, feature = "test-utils")), warn(unused_crate_dependencies))]

extern crate alloc;

/// Re-export commonly used types and traits.
pub mod prelude {
    pub use crate::{
        attributes::StatefulAttributesBuilder,
        errors::{PipelineError, PipelineErrorKind},
        pipeline::{DerivationPipeline, PipelineBuilder},
        sources::EthereumDataSource,
        traits::{ChainProvider, L2ChainProvider, OriginProvider, Pipeline, StepResult},
    };
}

pub mod attributes;
pub mod batch;
pub mod block;
pub mod errors;
pub mod pipeline;
pub mod sources;
pub mod stages;
pub mod traits;

#[cfg(feature = "metrics")]
pub mod metrics;

mod macros;
