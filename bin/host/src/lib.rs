#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

pub mod cli;
pub use cli::{init_tracing_subscriber, HostCli};

pub mod interop;
pub mod single;

pub mod eth;
pub mod fetcher;
pub mod kv;
pub mod preimage;
pub mod server;
