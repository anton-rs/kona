#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

// Temporarily pinned dependencies.
use foundry_fork_db as _;
use revm_inspectors as _;

// Used for op-test-vectors
use color_eyre as _;

use clap::Parser;

pub(crate) mod cli;
pub(crate) mod dn;
pub(crate) mod t8n;
pub(crate) mod traits;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::Cli::parse().init_telemetry()?.run().await
}
