//! Traits for resetting stages.

#![allow(unreachable_pub)]
#![allow(unused)]

use alloc::{boxed::Box, sync::Arc};
use async_trait::async_trait;
use kona_primitives::{BlockInfo, SystemConfig};
use spin::Mutex;

/// Provides the [BlockInfo] and [SystemConfig] for the stack to reset the stages.
#[async_trait]
pub trait ResetProvider {
    /// Returns the current [BlockInfo] for the pipeline to reset.
    async fn block_info(&self) -> BlockInfo;

    /// Returns the current [SystemConfig] for the pipeline to reset.
    async fn system_config(&self) -> SystemConfig;
}

/// TipState stores the tip information for the derivation pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct TipState {
    /// The origin [BlockInfo].
    /// This is used downstream by [kona_derive] to reset the origin
    /// of the [kona_derive::stages::BatchQueue] and l1 block list.
    origin: BlockInfo,
    /// The [SystemConfig] is used in two places.
    system_config: SystemConfig,
}

impl TipState {
    /// Creates a new [TipState].
    pub fn new(origin: BlockInfo, system_config: SystemConfig) -> Self {
        Self { origin, system_config }
    }

    /// Retrieves a copy of the [BlockInfo].
    pub fn origin(&self) -> BlockInfo {
        self.origin
    }

    /// Retrieves a copy of the [SystemConfig].
    pub fn system_config(&self) -> SystemConfig {
        self.system_config
    }

    /// Sets the block info.
    pub fn set_origin(&mut self, new_bi: BlockInfo) {
        self.origin = new_bi;
    }

    /// Sets the system config.
    pub fn set_system_config(&mut self, new_config: SystemConfig) {
        self.system_config = new_config;
    }
}

/// Wraps the [TipState] to implement the [ResetProvider] trait.
#[derive(Debug, Clone)]
pub struct WrappedTipState(Arc<Mutex<TipState>>);

impl WrappedTipState {
    /// Creates a new [ExExResetProvider].
    pub fn new(ts: Arc<Mutex<TipState>>) -> Self {
        Self(ts)
    }
}

#[async_trait]
impl ResetProvider for WrappedTipState {
    /// Returns the current [BlockInfo] for the pipeline to reset.
    async fn block_info(&self) -> BlockInfo {
        self.0.lock().origin()
    }

    /// Returns the current [SystemConfig] for the pipeline to reset.
    async fn system_config(&self) -> SystemConfig {
        self.0.lock().system_config()
    }
}
