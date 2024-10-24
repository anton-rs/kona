#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(any(test, feature = "metrics")), no_std)]
#![cfg_attr(not(any(test, feature = "test-utils")), warn(unused_crate_dependencies))]

extern crate alloc;
extern crate noerror as thiserror;

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
pub mod errors;
pub mod pipeline;
pub mod sources;
pub mod stages;
pub mod traits;

mod macros;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
