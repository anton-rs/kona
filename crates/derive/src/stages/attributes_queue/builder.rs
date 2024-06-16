//! The [`AttributesBuilder`] and it's default implementation.

use super::derive_deposits;
use crate::{
    params::SEQUENCER_FEE_VAULT_ADDRESS,
    traits::{ChainProvider, L2ChainProvider},
    types::{
        BlockID, BuilderError, EcotoneTransactionBuilder, L1BlockInfoTx, L2BlockInfo,
        L2PayloadAttributes, RawTransaction, RollupConfig,
    },
};
use alloc::{boxed::Box, fmt::Debug, sync::Arc, vec, vec::Vec};
use alloy_rlp::Encodable;
use async_trait::async_trait;
use op_alloy_consensus::Encodable2718;

/// The [AttributesBuilder] is responsible for preparing [L2PayloadAttributes]
/// that can be used to construct an L2 Block containing only deposits.
#[async_trait]
pub trait AttributesBuilder {
    /// Prepares a template [L2PayloadAttributes] that is ready to be used to build an L2 block.
    /// The block will contain deposits only, on top of the given L2 parent, with the L1 origin
    /// set to the given epoch.
    /// By default, the [L2PayloadAttributes] template will have `no_tx_pool` set to true,
    /// and no sequencer transactions. The caller has to modify the template to add transactions.
    /// This can be done by either setting the `no_tx_pool` to false as sequencer, or by appending
    /// batch transactions as the verifier.
    async fn prepare_payload_attributes(
        &mut self,
        l2_parent: L2BlockInfo,
        epoch: BlockID,
    ) -> Result<L2PayloadAttributes, BuilderError>;
}

/// A stateful implementation of the [AttributesBuilder].
#[derive(Debug, Default)]
pub struct StatefulAttributesBuilder<L1P, L2P>
where
    L1P: ChainProvider + Debug,
    L2P: L2ChainProvider + Debug,
{
    /// The rollup config.
    rollup_cfg: Arc<RollupConfig>,
    /// The system config fetcher.
    config_fetcher: L2P,
    /// The L1 receipts fetcher.
    receipts_fetcher: L1P,
}

impl<L1P, L2P> StatefulAttributesBuilder<L1P, L2P>
where
    L1P: ChainProvider + Debug,
    L2P: L2ChainProvider + Debug,
{
    /// Create a new [StatefulAttributesBuilder] with the given epoch.
    pub fn new(rcfg: Arc<RollupConfig>, sys_cfg_fetcher: L2P, receipts: L1P) -> Self {
        Self { rollup_cfg: rcfg, config_fetcher: sys_cfg_fetcher, receipts_fetcher: receipts }
    }
}

#[async_trait]
impl<L1P, L2P> AttributesBuilder for StatefulAttributesBuilder<L1P, L2P>
where
    L1P: ChainProvider + Debug + Send,
    L2P: L2ChainProvider + Debug + Send,
{
    async fn prepare_payload_attributes(
        &mut self,
        l2_parent: L2BlockInfo,
        epoch: BlockID,
    ) -> Result<L2PayloadAttributes, BuilderError> {
        let l1_header;
        let deposit_transactions: Vec<RawTransaction>;
        let mut sys_config = self
            .config_fetcher
            .system_config_by_number(l2_parent.block_info.number, self.rollup_cfg.clone())
            .await?;

        // If the L1 origin changed in this block, then we are in the first block of the epoch.
        // In this case we need to fetch all transaction receipts from the L1 origin block so
        // we can scan for user deposits.
        let sequence_number = if l2_parent.l1_origin.number != epoch.number {
            let header = self.receipts_fetcher.header_by_hash(epoch.hash).await?;
            if l2_parent.l1_origin.hash != header.parent_hash {
                return Err(BuilderError::BlockMismatchEpochReset(
                    epoch,
                    l2_parent.l1_origin,
                    header.parent_hash,
                ));
            }
            let receipts = self.receipts_fetcher.receipts_by_hash(epoch.hash).await?;
            sys_config.update_with_receipts(&receipts, &self.rollup_cfg, header.timestamp)?;
            let deposits =
                derive_deposits(epoch.hash, receipts, self.rollup_cfg.deposit_contract_address)
                    .await?;
            l1_header = header;
            deposit_transactions = deposits;
            0
        } else {
            #[allow(clippy::collapsible_else_if)]
            if l2_parent.l1_origin.hash != epoch.hash {
                return Err(BuilderError::BlockMismatch(epoch, l2_parent.l1_origin));
            }

            let header = self.receipts_fetcher.header_by_hash(epoch.hash).await?;
            l1_header = header;
            deposit_transactions = vec![];
            l2_parent.seq_num + 1
        };

        // Sanity check the L1 origin was correctly selected to maintain the time invariant
        // between L1 and L2.
        let next_l2_time = l2_parent.block_info.timestamp + self.rollup_cfg.block_time;
        if next_l2_time < l1_header.timestamp {
            return Err(BuilderError::BrokenTimeInvariant(
                l2_parent.l1_origin,
                next_l2_time,
                BlockID { hash: l1_header.hash_slow(), number: l1_header.number },
                l1_header.timestamp,
            ));
        }

        let mut upgrade_transactions: Vec<RawTransaction> = vec![];
        if self.rollup_cfg.is_ecotone_active(next_l2_time) &&
            !self.rollup_cfg.is_ecotone_active(l2_parent.block_info.timestamp)
        {
            upgrade_transactions =
                EcotoneTransactionBuilder::build_txs().map_err(BuilderError::Custom)?;
        }

        // Build and encode the L1 info transaction for the current payload.
        let (_, l1_info_tx_envelope) = L1BlockInfoTx::try_new_with_deposit_tx(
            &self.rollup_cfg,
            &sys_config,
            sequence_number,
            &l1_header,
            next_l2_time,
        )?;
        let mut encoded_l1_info_tx = Vec::with_capacity(l1_info_tx_envelope.length());
        l1_info_tx_envelope.encode_2718(&mut encoded_l1_info_tx);

        let mut txs =
            Vec::with_capacity(1 + deposit_transactions.len() + upgrade_transactions.len());
        txs.push(encoded_l1_info_tx.into());
        txs.extend(deposit_transactions);
        txs.extend(upgrade_transactions);

        let mut withdrawals = None;
        if self.rollup_cfg.is_canyon_active(next_l2_time) {
            withdrawals = Some(Vec::default());
        }

        let mut parent_beacon_root = None;
        if self.rollup_cfg.is_ecotone_active(next_l2_time) {
            // if the parent beacon root is not available, default to zero hash
            parent_beacon_root = Some(l1_header.parent_beacon_block_root.unwrap_or_default());
        }

        Ok(L2PayloadAttributes {
            timestamp: next_l2_time,
            prev_randao: l1_header.mix_hash,
            fee_recipient: SEQUENCER_FEE_VAULT_ADDRESS,
            transactions: txs,
            no_tx_pool: true,
            gas_limit: Some(u64::from_be_bytes(
                alloy_primitives::U64::from(sys_config.gas_limit).to_be_bytes(),
            )),
            withdrawals,
            parent_beacon_block_root: parent_beacon_root,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        stages::test_utils::MockSystemConfigL2Fetcher,
        traits::test_utils::TestChainProvider,
        types::{BlockInfo, SystemConfig},
    };
    use alloy_consensus::Header;
    use alloy_primitives::B256;

    #[tokio::test]
    async fn test_prepare_payload_block_mismatch_epoch_reset() {
        let cfg = Arc::new(RollupConfig::default());
        let l2_number = 1;
        let mut fetcher = MockSystemConfigL2Fetcher::default();
        fetcher.insert(l2_number, SystemConfig::default());
        let mut provider = TestChainProvider::default();
        let header = Header::default();
        let hash = header.hash_slow();
        provider.insert_header(hash, header);
        let mut builder = StatefulAttributesBuilder::new(cfg, fetcher, provider);
        let epoch = BlockID { hash, number: l2_number };
        let l2_parent = L2BlockInfo {
            block_info: BlockInfo { hash: B256::ZERO, number: l2_number, ..Default::default() },
            l1_origin: BlockID { hash: B256::left_padding_from(&[0xFF]), number: 2 },
            seq_num: 0,
        };
        // This should error because the l2 parent's l1_origin.hash should equal the epoch header
        // hash. Here we use the default header whose hash will not equal the custom `l2_hash`.
        let expected =
            BuilderError::BlockMismatchEpochReset(epoch, l2_parent.l1_origin, B256::default());
        let err = builder.prepare_payload_attributes(l2_parent, epoch).await.unwrap_err();
        assert_eq!(err, expected);
    }

    #[tokio::test]
    async fn test_prepare_payload_block_mismatch() {
        let cfg = Arc::new(RollupConfig::default());
        let l2_number = 1;
        let mut fetcher = MockSystemConfigL2Fetcher::default();
        fetcher.insert(l2_number, SystemConfig::default());
        let mut provider = TestChainProvider::default();
        let header = Header::default();
        let hash = header.hash_slow();
        provider.insert_header(hash, header);
        let mut builder = StatefulAttributesBuilder::new(cfg, fetcher, provider);
        let epoch = BlockID { hash, number: l2_number };
        let l2_parent = L2BlockInfo {
            block_info: BlockInfo { hash: B256::ZERO, number: l2_number, ..Default::default() },
            l1_origin: BlockID { hash: B256::ZERO, number: l2_number },
            seq_num: 0,
        };
        // This should error because the l2 parent's l1_origin.hash should equal the epoch hash
        // Here the default header is used whose hash will not equal the custom `l2_hash` above.
        let expected = BuilderError::BlockMismatch(epoch, l2_parent.l1_origin);
        let err = builder.prepare_payload_attributes(l2_parent, epoch).await.unwrap_err();
        assert_eq!(err, expected);
    }

    #[tokio::test]
    async fn test_prepare_payload_broken_time_invariant() {
        let block_time = 10;
        let timestamp = 100;
        let cfg = Arc::new(RollupConfig { block_time, ..Default::default() });
        let l2_number = 1;
        let mut fetcher = MockSystemConfigL2Fetcher::default();
        fetcher.insert(l2_number, SystemConfig::default());
        let mut provider = TestChainProvider::default();
        let header = Header { timestamp, ..Default::default() };
        let hash = header.hash_slow();
        provider.insert_header(hash, header);
        let mut builder = StatefulAttributesBuilder::new(cfg, fetcher, provider);
        let epoch = BlockID { hash, number: l2_number };
        let l2_parent = L2BlockInfo {
            block_info: BlockInfo { hash: B256::ZERO, number: l2_number, ..Default::default() },
            l1_origin: BlockID { hash, number: l2_number },
            seq_num: 0,
        };
        let next_l2_time = l2_parent.block_info.timestamp + block_time;
        let block_id = BlockID { hash, number: 0 };
        let expected = BuilderError::BrokenTimeInvariant(
            l2_parent.l1_origin,
            next_l2_time,
            block_id,
            timestamp,
        );
        let err = builder.prepare_payload_attributes(l2_parent, epoch).await.unwrap_err();
        assert_eq!(err, expected);
    }

    #[tokio::test]
    async fn test_prepare_payload_without_forks() {
        let block_time = 10;
        let timestamp = 100;
        let cfg = Arc::new(RollupConfig { block_time, ..Default::default() });
        let l2_number = 1;
        let mut fetcher = MockSystemConfigL2Fetcher::default();
        fetcher.insert(l2_number, SystemConfig::default());
        let mut provider = TestChainProvider::default();
        let header = Header { timestamp, ..Default::default() };
        let prev_randao = header.mix_hash;
        let hash = header.hash_slow();
        provider.insert_header(hash, header);
        let mut builder = StatefulAttributesBuilder::new(cfg, fetcher, provider);
        let epoch = BlockID { hash, number: l2_number };
        let l2_parent = L2BlockInfo {
            block_info: BlockInfo {
                hash: B256::ZERO,
                number: l2_number,
                timestamp,
                parent_hash: hash,
            },
            l1_origin: BlockID { hash, number: l2_number },
            seq_num: 0,
        };
        let next_l2_time = l2_parent.block_info.timestamp + block_time;
        let payload = builder.prepare_payload_attributes(l2_parent, epoch).await.unwrap();
        let expected = L2PayloadAttributes {
            timestamp: next_l2_time,
            prev_randao,
            fee_recipient: SEQUENCER_FEE_VAULT_ADDRESS,
            transactions: payload.transactions.clone(),
            no_tx_pool: true,
            gas_limit: Some(u64::from_be_bytes(
                alloy_primitives::U64::from(SystemConfig::default().gas_limit).to_be_bytes(),
            )),
            withdrawals: None,
            parent_beacon_block_root: None,
        };
        assert_eq!(payload, expected);
        assert_eq!(payload.transactions.len(), 1);
    }

    #[tokio::test]
    async fn test_prepare_payload_with_canyon() {
        let block_time = 10;
        let timestamp = 100;
        let cfg = Arc::new(RollupConfig { block_time, canyon_time: Some(0), ..Default::default() });
        let l2_number = 1;
        let mut fetcher = MockSystemConfigL2Fetcher::default();
        fetcher.insert(l2_number, SystemConfig::default());
        let mut provider = TestChainProvider::default();
        let header = Header { timestamp, ..Default::default() };
        let prev_randao = header.mix_hash;
        let hash = header.hash_slow();
        provider.insert_header(hash, header);
        let mut builder = StatefulAttributesBuilder::new(cfg, fetcher, provider);
        let epoch = BlockID { hash, number: l2_number };
        let l2_parent = L2BlockInfo {
            block_info: BlockInfo {
                hash: B256::ZERO,
                number: l2_number,
                timestamp,
                parent_hash: hash,
            },
            l1_origin: BlockID { hash, number: l2_number },
            seq_num: 0,
        };
        let next_l2_time = l2_parent.block_info.timestamp + block_time;
        let payload = builder.prepare_payload_attributes(l2_parent, epoch).await.unwrap();
        let expected = L2PayloadAttributes {
            timestamp: next_l2_time,
            prev_randao,
            fee_recipient: SEQUENCER_FEE_VAULT_ADDRESS,
            transactions: payload.transactions.clone(),
            no_tx_pool: true,
            gas_limit: Some(u64::from_be_bytes(
                alloy_primitives::U64::from(SystemConfig::default().gas_limit).to_be_bytes(),
            )),
            withdrawals: Some(Vec::default()),
            parent_beacon_block_root: None,
        };
        assert_eq!(payload, expected);
        assert_eq!(payload.transactions.len(), 1);
    }

    #[tokio::test]
    async fn test_prepare_payload_with_ecotone() {
        let block_time = 2;
        let timestamp = 100;
        let cfg =
            Arc::new(RollupConfig { block_time, ecotone_time: Some(102), ..Default::default() });
        let l2_number = 1;
        let mut fetcher = MockSystemConfigL2Fetcher::default();
        fetcher.insert(l2_number, SystemConfig::default());
        let mut provider = TestChainProvider::default();
        let header = Header { timestamp, ..Default::default() };
        let parent_beacon_block_root = Some(header.parent_beacon_block_root.unwrap_or_default());
        let prev_randao = header.mix_hash;
        let hash = header.hash_slow();
        provider.insert_header(hash, header);
        let mut builder = StatefulAttributesBuilder::new(cfg, fetcher, provider);
        let epoch = BlockID { hash, number: l2_number };
        let l2_parent = L2BlockInfo {
            block_info: BlockInfo {
                hash: B256::ZERO,
                number: l2_number,
                timestamp,
                parent_hash: hash,
            },
            l1_origin: BlockID { hash, number: l2_number },
            seq_num: 0,
        };
        let next_l2_time = l2_parent.block_info.timestamp + block_time;
        let payload = builder.prepare_payload_attributes(l2_parent, epoch).await.unwrap();
        let expected = L2PayloadAttributes {
            timestamp: next_l2_time,
            prev_randao,
            fee_recipient: SEQUENCER_FEE_VAULT_ADDRESS,
            transactions: payload.transactions.clone(),
            no_tx_pool: true,
            gas_limit: Some(u64::from_be_bytes(
                alloy_primitives::U64::from(SystemConfig::default().gas_limit).to_be_bytes(),
            )),
            withdrawals: None,
            parent_beacon_block_root,
        };
        assert_eq!(payload, expected);
        assert_eq!(payload.transactions.len(), 7);
    }
}
