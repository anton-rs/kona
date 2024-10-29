#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(test), no_std)]

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
pub mod errors;
pub mod pipeline;
pub mod sources;
pub mod stages;
pub mod traits;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
