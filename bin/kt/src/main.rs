#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use clap::Parser;
use kona_primitives::{Blob, L2BlockInfo, L2PayloadAttributes, RollupConfig, SystemConfig};

/// A local derivation fixture typed with `kona_derive` types.
pub type LocalDerivationFixture = op_test_vectors::derivation::DerivationFixture<
    RollupConfig,
    L2PayloadAttributes,
    SystemConfig,
    L2BlockInfo,
    Blob,
>;

pub(crate) mod cli;
pub(crate) mod dn;
pub(crate) mod traits;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::Cli::parse().init_telemetry()?.run().await
}
