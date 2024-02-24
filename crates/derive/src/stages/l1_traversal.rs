//! Contains the L1 traversal stage of the derivation pipeline.

#![allow(dead_code, unused)]

use crate::{
    traits::{BlockByNumberProvider, ResettableStage},
    types::SystemConfig,
};
use anyhow::Result;

/// The L1 traversal stage of the derivation pipeline.
#[derive(Debug, Clone, Copy)]
pub struct L1Traversal<F: BlockByNumberProvider> {
    /// The current block in the traversal stage.
    block: (),
    /// The data source for the traversal stage.
    data_source: F,
    /// Signals whether or not the traversal stage has been completed.
    done: bool,
    /// The system config
    system_config: SystemConfig,
    /// The rollup config
    rollup_config: (),
}

impl<F: BlockByNumberProvider> L1Traversal<F> {
    /// Creates a new [L1Traversal] instance.
    pub fn new(data_source: F) -> Self {
        Self {
            block: (),
            data_source,
            done: false,
            system_config: SystemConfig::default(),
            rollup_config: (),
        }
    }

    /// Returns the next L1 block in the traversal stage, if the stage has not been completed. This function can only
    /// be called once, and will return `None` on subsequent calls unless the stage is reset.
    pub fn next_l1_block(&mut self) -> Option<()> {
        if !self.done {
            self.done = true;
            todo!("Return block once we have a `Block` type");
        } else {
            None
        }
    }

    /// Advances the internal state of the [L1Traversal] stage to the next L1 block.
    pub async fn advance_l1_block(&mut self) -> Result<()> {
        todo!("Once we have a `Block` type and a `RollupConfig` type");
    }
}

impl<F: BlockByNumberProvider> ResettableStage for L1Traversal<F> {
    fn reset(&mut self) -> Result<()> {
        todo!()
    }
}
