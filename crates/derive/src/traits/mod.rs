//! This module contains all of the traits describing functionality of portions of the derivation pipeline.

mod data_sources;
pub use data_sources::BlockByNumberProvider;

mod stages;
pub use stages::ResettableStage;
