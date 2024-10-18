//! This module contains all of the traits describing functionality of portions of the derivation
//! pipeline.

mod pipeline;
pub use pipeline::{ActivationSignal, Pipeline, ResetSignal, Signal, StepResult};

mod attributes;
pub use attributes::{AttributesBuilder, NextAttributes};

mod data_sources;
pub use data_sources::{AsyncIterator, BlobProvider, DataAvailabilityProvider};

mod reset;
pub use reset::ResetProvider;

mod stages;
pub use stages::{OriginAdvancer, OriginProvider, SignalReceiver};
