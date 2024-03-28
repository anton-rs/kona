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
            block: Some(BlockInfo::default()),
            data_source,
            done: false,
            system_config: SystemConfig::default(),
            rollup_config: cfg,
        }
    }

    /// Retrieves a reference to the inner data source of the [L1Traversal] stage.
    pub fn data_source(&self) -> &F {
        &self.data_source
    }

    /// Returns the next L1 block in the traversal stage, if the stage has not been completed.
    /// This function can only be called once, and will return `None` on subsequent calls
    /// unless the stage is reset.
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
pub(crate) mod tests {
    use super::*;
    use crate::traits::test_utils::TestChainProvider;
    use crate::types::{Receipt, CONFIG_UPDATE_EVENT_VERSION_0, CONFIG_UPDATE_TOPIC};
    use alloc::vec;
    use alloy_primitives::{address, b256, hex, Address, Bytes, Log, LogData, B256};

    const L1_SYS_CONFIG_ADDR: Address = address!("1337000000000000000000000000000000000000");

    fn new_update_batcher_log() -> Log {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000000");
        Log {
            address: L1_SYS_CONFIG_ADDR,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        }
    }

    pub(crate) fn new_test_traversal(
        blocks: bool,
        receipts: bool,
    ) -> L1Traversal<TestChainProvider> {
        let mut provider = TestChainProvider::default();
        let rollup_config = RollupConfig {
            l1_system_config_address: L1_SYS_CONFIG_ADDR,
            ..RollupConfig::default()
        };
        let block = BlockInfo::default();
        if blocks {
            provider.insert_block(0, block);
            provider.insert_block(1, block);
        }
        if receipts {
            let mut receipt = Receipt {
                success: true,
                ..Receipt::default()
            };
            let bad = Log::new(
                Address::from([2; 20]),
                vec![CONFIG_UPDATE_TOPIC, B256::default()],
                Bytes::default(),
            )
            .unwrap();
            receipt.logs = vec![new_update_batcher_log(), bad, new_update_batcher_log()];
            let receipts = vec![receipt.clone(), Receipt::default(), receipt];
            provider.insert_receipts(block.hash, receipts);
        }
        L1Traversal::new(provider, rollup_config)
    }

    #[tokio::test]
    async fn test_l1_traversal() {
        let mut traversal = new_test_traversal(true, true);
        assert_eq!(
            traversal.next_l1_block().unwrap(),
            Some(BlockInfo::default())
        );
        assert_eq!(traversal.next_l1_block().unwrap_err(), StageError::Eof);
        assert!(traversal.advance_l1_block().await.is_ok());
    }

    #[tokio::test]
    async fn test_l1_traversal_missing_receipts() {
        let mut traversal = new_test_traversal(true, false);
        assert_eq!(
            traversal.next_l1_block().unwrap(),
            Some(BlockInfo::default())
        );
        assert_eq!(traversal.next_l1_block().unwrap_err(), StageError::Eof);
        matches!(
            traversal.advance_l1_block().await.unwrap_err(),
            StageError::Custom(_)
        );
    }

    #[tokio::test]
    async fn test_l1_traversal_missing_blocks() {
        let mut traversal = new_test_traversal(false, false);
        assert_eq!(
            traversal.next_l1_block().unwrap(),
            Some(BlockInfo::default())
        );
        assert_eq!(traversal.next_l1_block().unwrap_err(), StageError::Eof);
        matches!(
            traversal.advance_l1_block().await.unwrap_err(),
            StageError::Custom(_)
        );
    }

    #[tokio::test]
    async fn test_system_config_updated() {
        let mut traversal = new_test_traversal(true, true);
        assert_eq!(
            traversal.next_l1_block().unwrap(),
            Some(BlockInfo::default())
        );
        assert_eq!(traversal.next_l1_block().unwrap_err(), StageError::Eof);
        assert!(traversal.advance_l1_block().await.is_ok());
        let expected = address!("000000000000000000000000000000000000bEEF");
        assert_eq!(traversal.system_config.batcher_addr, expected);
    }
}
