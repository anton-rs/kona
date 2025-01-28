#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/op-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/op-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[macro_use]
extern crate tracing;

mod core;
pub use core::Driver;

mod heads;
pub use heads::{L2ChainHeads, SafetyLabel};

mod errors;
pub use errors::{DriverError, DriverResult};

mod engine;
pub use engine::{EngineController, OpExecutionPayloadEnvelope};
