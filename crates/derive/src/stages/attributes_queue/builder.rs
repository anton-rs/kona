//! The [`AttributesBuilder`] and it's default implementation.

use crate::{
    params::SEQUENCER_FEE_VAULT_ADDRESS,
    types::{
        BlockID, BlockInfo, BuilderError, L2BlockInfo, PayloadAttributes, RawTransaction, Receipt,
        RollupConfig, SystemConfig,
    },
};
use alloc::{fmt::Debug, sync::Arc, vec, vec::Vec};
use alloy_primitives::B256;

/// The [AttributesBuilder] is responsible for preparing [PayloadAttributes]
/// that can be used to construct an L2 Block containing only deposits.
pub trait AttributesBuilder {
    /// Prepares a template [PayloadAttributes] that is ready to be used to build an L2 block.
    /// The block will contain deposits only, on top of the given L2 parent, with the L1 origin
    /// set to the given epoch.
    /// By default, the [PayloadAttributes] template will have `no_tx_pool` set to true,
    /// and no sequencer transactions. The caller has to modify the template to add transactions.
    /// This can be done by either setting the `no_tx_pool` to false as sequencer, or by appending
    /// batch transactions as the verifier.
    fn prepare_payload_attributes(
        &mut self,
        l2_parent: L2BlockInfo,
        epoch: BlockID,
    ) -> Result<PayloadAttributes, BuilderError>;
}

/// The [L1ReceiptsFetcher] fetches L1 Header Info and [Receipt]s for
/// the payload attributes derivation (the info tx and deposits).
pub trait L1ReceiptsFetcher {
    /// Fetch the L1 Header Info by hash.
    fn info_by_hash(&self, hash: B256) -> anyhow::Result<BlockInfo>;

    /// Fetch the [Receipt]s by block hash.
    fn fetch_receipts(&self, block_hash: B256) -> anyhow::Result<(BlockInfo, Vec<Receipt>)>;
}

/// The [SystemConfigL2Fetcher] fetches the system config by L2 hash.
pub trait SystemConfigL2Fetcher {
    /// Fetch the system config by L2 hash.
    fn system_config_by_l2_hash(&self, hash: B256) -> anyhow::Result<SystemConfig>;
}

/// A stateful implementation of the [AttributesBuilder].
#[derive(Debug, Default)]
pub struct StatefulAttributesBuilder<S, R>
where
    S: SystemConfigL2Fetcher + Debug,
    R: L1ReceiptsFetcher + Debug,
{
    /// The rollup config.
    rollup_cfg: Arc<RollupConfig>,
    /// The system config fetcher.
    config_fetcher: S,
    /// The L1 receipts fetcher.
    receipts_fetcher: R,
}

impl<S, R> StatefulAttributesBuilder<S, R>
where
    S: SystemConfigL2Fetcher + Debug,
    R: L1ReceiptsFetcher + Debug,
{
    /// Create a new [StatefulAttributesBuilder] with the given epoch.
    pub fn new(rcfg: Arc<RollupConfig>, cfg: S, receipts: R) -> Self {
        Self { rollup_cfg: rcfg, config_fetcher: cfg, receipts_fetcher: receipts }
    }
}

impl<S, R> AttributesBuilder for StatefulAttributesBuilder<S, R>
where
    S: SystemConfigL2Fetcher + Debug,
    R: L1ReceiptsFetcher + Debug,
{
    fn prepare_payload_attributes(
        &mut self,
        l2_parent: L2BlockInfo,
        epoch: BlockID,
    ) -> Result<PayloadAttributes, BuilderError> {
        let l1_info;
        let mut deposit_transactions: Vec<RawTransaction> = vec![];
        // let mut sequence_number = 0u64;
        let mut sys_config =
            self.config_fetcher.system_config_by_l2_hash(l2_parent.block_info.hash)?;

        // If the L1 origin changed in this block, then we are in the first block of the epoch.
        // In this case we need to fetch all transaction receipts from the L1 origin block so
        // we can scan for user deposits.
        if l2_parent.l1_origin.number != epoch.number {
            let (info, receipts) = self.receipts_fetcher.fetch_receipts(epoch.hash)?;
            if l2_parent.l1_origin.hash != info.parent_hash {
                return Err(BuilderError::BlockMismatchEpochReset(
                    epoch,
                    l2_parent.l1_origin,
                    info.parent_hash,
                ));
            }

            // let deposits = derive_deposits(receipts, sys_config.deposit_contract_address)?;
            sys_config.update_with_receipts(&receipts, &self.rollup_cfg, info.timestamp)?;

            l1_info = info;
            // deposit_transactions = deposits;
            // sequence_number = 0;
        } else {
            #[allow(clippy::collapsible_else_if)]
            if l2_parent.l1_origin.hash != epoch.hash {
                return Err(BuilderError::BlockMismatch(epoch, l2_parent.l1_origin));
            }

            let info = self.receipts_fetcher.info_by_hash(epoch.hash)?;
            l1_info = info;
            deposit_transactions = vec![];
            // sequence_number = l2_parent.seq_num + 1;
        }

        // Sanity check the L1 origin was correctly selected to maintain the time invariant
        // between L1 and L2.
        let next_l2_time = l2_parent.block_info.timestamp + self.rollup_cfg.block_time;
        if next_l2_time < l1_info.timestamp {
            return Err(BuilderError::BrokenTimeInvariant(
                l2_parent.l1_origin,
                next_l2_time,
                l1_info.id(),
                l1_info.timestamp,
            ));
        }

        // let mut upgrade_transactions: Vec<Bytes> = vec![];
        // if self.rollup_cfg.is_ecotone_active(next_l2_time) {
        //     upgrade_transactions =
        // EcotoneTransactionBuilder::build_txs().map_err(BuilderError::Custom)?; }

        // let l1_info_tx = l1_info_deposit_bytes(self.rollup_cfg, sys_config, sequence_number,
        // l1_info, next_l2_time)?;

        let mut txs = vec![];
        // txs.push(l1_info_tx);
        txs.extend(deposit_transactions);
        // txs.extend(upgrade_transactions);

        let withdrawals = None;
        if self.rollup_cfg.is_canyon_active(next_l2_time) {
            // withdrawals = Some(Withdrawals::default());
        }

        let parent_beacon_root = None;
        if self.rollup_cfg.is_ecotone_active(next_l2_time) {
            // parent_beacon_root = Some(l1_info.parent_beacon_root().unwrap_or_default());
            // if the parent beacon root is not available, default to zero hash
        }

        Ok(PayloadAttributes {
            timestamp: next_l2_time,
            // TODO: The mix digest is pulled from the l1 info, which is a **full** block.
            //       Currently, the l1_info is only the minimal `BlockInfo` defined in our types.
            prev_randao: B256::default(),
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
