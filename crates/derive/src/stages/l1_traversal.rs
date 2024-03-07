//! Contains the L1 traversal stage of the derivation pipeline.

use crate::{
    traits::{ChainProvider, ResettableStage},
    types::{BlockInfo, RollupConfig, StageError, StageResult, SystemConfig},
};
use alloc::boxed::Box;
use anyhow::anyhow;
use async_trait::async_trait;

/// The L1 traversal stage of the derivation pipeline.
#[derive(Debug, Clone, Copy)]
pub struct L1Traversal<Provider: ChainProvider> {
    /// The current block in the traversal stage.
    block: Option<BlockInfo>,
    /// The data source for the traversal stage.
    data_source: Provider,
    /// Signals whether or not the traversal stage has been completed.
    done: bool,
    /// The system config
    pub system_config: SystemConfig,
    /// The rollup config
    pub rollup_config: RollupConfig,
}

impl<F: ChainProvider> L1Traversal<F> {
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
    pub fn next_l1_block(&mut self) -> StageResult<Option<BlockInfo>> {
        if !self.done {
            self.done = true;
            Ok(self.block)
        } else {
            Err(StageError::Eof)
        }
    }

    /// Returns the current L1 block in the traversal stage, if it exists.
    pub fn origin(&self) -> Option<&BlockInfo> {
        self.block.as_ref()
    }

    /// Advances the internal state of the [L1Traversal] stage to the next L1 block.
    pub async fn advance_l1_block(&mut self) -> StageResult<()> {
        // Pull the next block or return EOF which has special
        // handling further up the pipeline.
        let block = self.block.ok_or(StageError::Eof)?;
        let next_l1_origin = self
            .data_source
            .block_info_by_number(block.number + 1)
            .await?;

        // Check for reorgs
        if block.hash != next_l1_origin.parent_hash {
            return Err(anyhow!(
                "Detected L1 reorg from {} to {} with conflicting parent",
                block.hash,
                next_l1_origin.hash
            )
            .into());
        }

        // Fetch receipts.
        let receipts = self
            .data_source
            .receipts_by_hash(next_l1_origin.hash)
            .await?;
        self.system_config.update_with_receipts(
            receipts.as_slice(),
            &self.rollup_config,
            next_l1_origin.timestamp,
        )?;

        self.block = Some(next_l1_origin);
        self.done = false;
        Ok(())
    }
}

#[async_trait]
impl<F: ChainProvider + Send> ResettableStage for L1Traversal<F> {
    async fn reset(&mut self, base: BlockInfo, cfg: SystemConfig) -> StageResult<()> {
        self.block = Some(base);
        self.done = false;
        self.system_config = cfg;
        Err(StageError::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::test_utils::TestChainProvider;

    #[tokio::test]
    async fn test_l1_traversal() {
        let mut provider = TestChainProvider::default();
        let block = BlockInfo {
            number: 0,
            hash: Default::default(),
            parent_hash: Default::default(),
            timestamp: 0,
        };
        provider.insert_block(0, block);
        let mut traversal = L1Traversal::new(provider, RollupConfig::default());
        assert_eq!(traversal.next_l1_block().unwrap(), Some(block));
        assert_eq!(traversal.next_l1_block().unwrap_err(), StageError::Eof);
        assert_eq!(
            traversal.advance_l1_block().await.unwrap_err(),
            StageError::Eof
        );
        assert_eq!(
            traversal.advance_l1_block().await.unwrap_err(),
            StageError::Eof
        );
        assert_eq!(traversal.next_l1_block().unwrap(), Some(block));
    }
}
