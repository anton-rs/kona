//! Contains the L1 traversal stage of the derivation pipeline.

#![allow(dead_code, unused)]

use crate::{
    traits::{BlockByNumberProvider, ResettableStage},
    types::{BlockInfo, RollupConfig, SystemConfig},
};
use anyhow::{anyhow, bail, Result};

/// The L1 traversal stage of the derivation pipeline.
#[derive(Debug, Clone, Copy)]
pub struct L1Traversal<F: BlockByNumberProvider> {
    /// The current block in the traversal stage.
    block: Option<BlockInfo>,
    /// The data source for the traversal stage.
    data_source: F,
    /// Signals whether or not the traversal stage has been completed.
    done: bool,
    /// The system config
    system_config: SystemConfig,
    /// The rollup config
    rollup_config: RollupConfig,
}

impl<F: BlockByNumberProvider> L1Traversal<F> {
    /// Creates a new [L1Traversal] instance.
    pub fn new(data_source: F, cfg: RollupConfig) -> Self {
        Self {
            block: None,
            data_source,
            done: false,
            system_config: SystemConfig::default(),
            rollup_config: cfg,
        }
    }

    /// Returns the next L1 block in the traversal stage, if the stage has not been completed. This function can only
    /// be called once, and will return `None` on subsequent calls unless the stage is reset.
    pub fn next_l1_block(&mut self) -> Option<BlockInfo> {
        if !self.done {
            self.done = true;
            self.block
        } else {
            None
        }
    }

    /// Advances the internal state of the [L1Traversal] stage to the next L1 block.
    pub async fn advance_l1_block(&mut self) -> Result<()> {
        let block = self.block.ok_or(anyhow!("No block to advance from"))?;
        let next_l1_origin = self.data_source.block_by_number(block.number + 1).await?;

        // Check for reorgs
        if block.hash != next_l1_origin.parent_hash {
            bail!(
                "Detected L1 reorg from {} to {} with conflicting parent",
                block.hash,
                next_l1_origin.hash
            );
        }

        // Fetch receipts.
        todo!("Once we have a `Receipt` type");
    }
}

impl<F: BlockByNumberProvider> ResettableStage for L1Traversal<F> {
    fn reset(&mut self) -> Result<()> {
        todo!()
    }
}
