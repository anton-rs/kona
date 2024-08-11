//! Contains the [L1Traversal] stage of the derivation pipeline.

use crate::{
    stages::L1RetrievalProvider,
    traits::{ChainProvider, OriginAdvancer, OriginProvider, ResettableStage},
    types::{BlockInfo, RollupConfig, StageError, StageResult, SystemConfig},
};
use alloc::{boxed::Box, sync::Arc};
use alloy_primitives::Address;
use async_trait::async_trait;
use tracing::warn;

/// The [L1Traversal] stage of the derivation pipeline.
///
/// This stage sits at the bottom of the pipeline, holding a handle to the data source
/// (a [ChainProvider] implementation) and the current L1 [BlockInfo] in the pipeline,
/// which are used to traverse the L1 chain. When the [L1Traversal] stage is advanced,
/// it fetches the next L1 [BlockInfo] from the data source and updates the [SystemConfig]
/// with the receipts from the block.
#[derive(Debug, Clone)]
pub struct L1Traversal<Provider: ChainProvider> {
    /// The current block in the traversal stage.
    pub block: Option<BlockInfo>,
    /// The data source for the traversal stage.
    data_source: Provider,
    /// Signals whether or not the traversal stage is complete.
    done: bool,
    /// The system config.
    pub system_config: SystemConfig,
    /// A reference to the rollup config.
    pub rollup_config: Arc<RollupConfig>,
}

#[async_trait]
impl<F: ChainProvider + Send> L1RetrievalProvider for L1Traversal<F> {
    fn batcher_addr(&self) -> Address {
        self.system_config.batcher_address
    }

    async fn next_l1_block(&mut self) -> StageResult<Option<BlockInfo>> {
        if !self.done {
            self.done = true;
            Ok(self.block)
        } else {
            Err(StageError::Eof)
        }
    }
}

impl<F: ChainProvider> L1Traversal<F> {
    /// Creates a new [L1Traversal] instance.
    pub fn new(data_source: F, cfg: Arc<RollupConfig>) -> Self {
        crate::set!(STAGE_RESETS, 0, &["l1-traversal"]);
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
}

#[async_trait]
impl<F: ChainProvider + Send> OriginAdvancer for L1Traversal<F> {
    /// Advances the internal state of the [L1Traversal] stage to the next L1 block.
    /// This function fetches the next L1 [BlockInfo] from the data source and updates the
    /// [SystemConfig] with the receipts from the block.
    async fn advance_origin(&mut self) -> StageResult<()> {
        // Pull the next block or return EOF.
        // StageError::EOF has special handling further up the pipeline.
        let block = match self.block {
            Some(block) => block,
            None => {
                warn!(target: "l1-traversal",  "Missing current block, can't advance origin with no reference.");
                return Err(StageError::Eof);
            }
        };
        let next_l1_origin = match self.data_source.block_info_by_number(block.number + 1).await {
            Ok(block) => block,
            Err(e) => return Err(StageError::BlockInfoFetch(e)),
        };

        // Check block hashes for reorgs.
        if block.hash != next_l1_origin.parent_hash {
            return Err(StageError::ReorgDetected(block.hash, next_l1_origin.parent_hash));
        }

        // Fetch receipts for the next l1 block and update the system config.
        let receipts = match self.data_source.receipts_by_hash(next_l1_origin.hash).await {
            Ok(receipts) => receipts,
            Err(e) => return Err(StageError::ReceiptFetch(e)),
        };

        if let Err(e) = self.system_config.update_with_receipts(
            receipts.as_slice(),
            &self.rollup_config,
            next_l1_origin.timestamp,
        ) {
            return Err(StageError::SystemConfigUpdate(e));
        }

        crate::set!(ORIGIN_GAUGE, next_l1_origin.number as i64);
        self.block = Some(next_l1_origin);
        self.done = false;
        Ok(())
    }
}

impl<F: ChainProvider> OriginProvider for L1Traversal<F> {
    fn origin(&self) -> Option<BlockInfo> {
        self.block
    }
}

#[async_trait]
impl<F: ChainProvider + Send> ResettableStage for L1Traversal<F> {
    async fn reset(&mut self, base: BlockInfo, cfg: &SystemConfig) -> StageResult<()> {
        self.block = Some(base);
        self.done = false;
        self.system_config = cfg.clone();
        crate::inc!(STAGE_RESETS, &["l1-traversal"]);
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        params::{CONFIG_UPDATE_EVENT_VERSION_0, CONFIG_UPDATE_TOPIC},
        traits::test_utils::TestChainProvider,
    };
    use alloc::vec;
    use alloy_consensus::Receipt;
    use alloy_primitives::{address, b256, hex, Bytes, Log, LogData, B256};

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

    pub(crate) fn new_receipts() -> alloc::vec::Vec<Receipt> {
        let mut receipt =
            Receipt { status: alloy_consensus::Eip658Value::Eip658(true), ..Receipt::default() };
        let bad = Log::new(
            Address::from([2; 20]),
            vec![CONFIG_UPDATE_TOPIC, B256::default()],
            Bytes::default(),
        )
        .unwrap();
        receipt.logs = vec![new_update_batcher_log(), bad, new_update_batcher_log()];
        vec![receipt.clone(), Receipt::default(), receipt]
    }

    pub(crate) fn new_test_traversal(
        blocks: alloc::vec::Vec<BlockInfo>,
        receipts: alloc::vec::Vec<Receipt>,
    ) -> L1Traversal<TestChainProvider> {
        let mut provider = TestChainProvider::default();
        let rollup_config = RollupConfig {
            l1_system_config_address: L1_SYS_CONFIG_ADDR,
            ..RollupConfig::default()
        };
        for (i, block) in blocks.iter().enumerate() {
            provider.insert_block(i as u64, *block);
        }
        for (i, receipt) in receipts.iter().enumerate() {
            let hash = blocks.get(i).map(|b| b.hash).unwrap_or_default();
            provider.insert_receipts(hash, vec![receipt.clone()]);
        }
        L1Traversal::new(provider, Arc::new(rollup_config))
    }

    pub(crate) fn new_populated_test_traversal() -> L1Traversal<TestChainProvider> {
        let blocks = vec![BlockInfo::default(), BlockInfo::default()];
        let receipts = new_receipts();
        new_test_traversal(blocks, receipts)
    }

    #[tokio::test]
    async fn test_l1_traversal() {
        let blocks = vec![BlockInfo::default(), BlockInfo::default()];
        let receipts = new_receipts();
        let mut traversal = new_test_traversal(blocks, receipts);
        assert_eq!(traversal.next_l1_block().await.unwrap(), Some(BlockInfo::default()));
        assert_eq!(traversal.next_l1_block().await.unwrap_err(), StageError::Eof);
        assert!(traversal.advance_origin().await.is_ok());
    }

    #[tokio::test]
    async fn test_l1_traversal_missing_receipts() {
        let blocks = vec![BlockInfo::default(), BlockInfo::default()];
        let mut traversal = new_test_traversal(blocks, vec![]);
        assert_eq!(traversal.next_l1_block().await.unwrap(), Some(BlockInfo::default()));
        assert_eq!(traversal.next_l1_block().await.unwrap_err(), StageError::Eof);
        matches!(traversal.advance_origin().await.unwrap_err(), StageError::ReceiptFetch(_));
    }

    #[tokio::test]
    async fn test_l1_traversal_reorgs() {
        let hash = b256!("3333333333333333333333333333333333333333333333333333333333333333");
        let block = BlockInfo { hash, ..BlockInfo::default() };
        let blocks = vec![block, block];
        let receipts = new_receipts();
        let mut traversal = new_test_traversal(blocks, receipts);
        assert!(traversal.advance_origin().await.is_ok());
        let err = traversal.advance_origin().await.unwrap_err();
        assert_eq!(err, StageError::ReorgDetected(block.hash, block.parent_hash));
    }

    #[tokio::test]
    async fn test_l1_traversal_missing_blocks() {
        let mut traversal = new_test_traversal(vec![], vec![]);
        assert_eq!(traversal.next_l1_block().await.unwrap(), Some(BlockInfo::default()));
        assert_eq!(traversal.next_l1_block().await.unwrap_err(), StageError::Eof);
        matches!(traversal.advance_origin().await.unwrap_err(), StageError::BlockInfoFetch(_));
    }

    #[tokio::test]
    async fn test_l1_traversal_system_config_update_fails() {
        let first = b256!("3333333333333333333333333333333333333333333333333333333333333333");
        let second = b256!("4444444444444444444444444444444444444444444444444444444444444444");
        let block1 = BlockInfo { hash: first, ..BlockInfo::default() };
        let block2 = BlockInfo { hash: second, ..BlockInfo::default() };
        let blocks = vec![block1, block2];
        let receipts = new_receipts();
        let mut traversal = new_test_traversal(blocks, receipts);
        assert!(traversal.advance_origin().await.is_ok());
        // Only the second block should fail since the second receipt
        // contains invalid logs that will error for a system config update.
        let err = traversal.advance_origin().await.unwrap_err();
        matches!(err, StageError::SystemConfigUpdate(_));
    }

    #[tokio::test]
    async fn test_l1_traversal_system_config_updated() {
        let blocks = vec![BlockInfo::default(), BlockInfo::default()];
        let receipts = new_receipts();
        let mut traversal = new_test_traversal(blocks, receipts);
        assert_eq!(traversal.next_l1_block().await.unwrap(), Some(BlockInfo::default()));
        assert_eq!(traversal.next_l1_block().await.unwrap_err(), StageError::Eof);
        assert!(traversal.advance_origin().await.is_ok());
        let expected = address!("000000000000000000000000000000000000bEEF");
        assert_eq!(traversal.system_config.batcher_address, expected);
    }
}
