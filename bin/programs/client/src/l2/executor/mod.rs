//! The block executor for the L2 client program. Operates off of a [TrieDB] backed [State],
//! allowing for stateless block execution of OP Stack blocks.

use alloc::{sync::Arc, vec::Vec};
use alloy_consensus::{Header, Sealable, Sealed, EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};
use alloy_eips::eip2718::{Decodable2718, Encodable2718};
use alloy_primitives::{address, Bytes, TxKind, B256, U256};
use anyhow::{anyhow, Result};
use kona_derive::types::{L2PayloadAttributes, RawTransaction, RollupConfig};
use kona_mpt::{ordered_trie_with_encoder, TrieDB, TrieDBFetcher};
use op_alloy_consensus::{OpReceipt, OpReceiptEnvelope, OpReceiptWithBloom, OpTxEnvelope};
use revm::{
    db::{states::bundle_state::BundleRetention, State},
    primitives::{
        calc_excess_blob_gas, BlobExcessGasAndPrice, BlockEnv, CfgEnv, CfgEnvWithHandlerCfg,
        EnvWithHandlerCfg, OptimismFields, SpecId, TransactTo, TxEnv,
    },
    Evm, StateBuilder,
};

mod fetcher;
pub use fetcher::TrieDBProvider;

mod eip4788;
pub(crate) use eip4788::pre_block_beacon_root_contract_call;

mod canyon;
pub(crate) use canyon::ensure_create2_deployer_canyon;

mod util;
pub(crate) use util::{logs_bloom, wrap_receipt_with_bloom};

/// The block executor for the L2 client program. Operates off of a [TrieDB] backed [State],
/// allowing for stateless block execution of OP Stack blocks.
#[derive(Debug)]
pub struct StatelessL2BlockExecutor<F>
where
    F: TrieDBFetcher,
{
    /// The [RollupConfig].
    config: Arc<RollupConfig>,
    /// The parent header
    parent_header: Sealed<Header>,
    /// The inner state database component.
    state: State<TrieDB<F>>,
}

impl<F> StatelessL2BlockExecutor<F>
where
    F: TrieDBFetcher,
{
    /// Constructs a new [StatelessL2BlockExecutor] with the given starting state root, parent hash,
    /// and [TrieDBFetcher].
    pub fn new(
        config: Arc<RollupConfig>,
        starting_state_root: B256,
        parent_header: Sealed<Header>,
        fetcher: F,
    ) -> Self {
        let trie_db = TrieDB::new(starting_state_root, parent_header.seal(), fetcher);
        let state = StateBuilder::new_with_database(trie_db).with_bundle_update().build();
        Self { config, parent_header, state }
    }
}

impl<F> StatelessL2BlockExecutor<F>
where
    F: TrieDBFetcher,
{
    /// Executes the given block, returning the resulting state root.
    ///
    /// ## Steps
    /// 1. Prepare the block environment.
    /// 2. Apply the pre-block EIP-4788 contract call.
    /// 3. Prepare the EVM with the given L2 execution payload in the block environment.
    ///     - Reject any EIP-4844 transactions, as they are not supported on the OP Stack.
    ///     - If the transaction is a deposit, cache the depositor account prior to execution.
    ///     - Construct the EVM with the given configuration.
    ///     - Execute the transaction.
    ///     - Accumulate the gas used by the transaction to the block-scoped cumulative gas used
    ///       counter.
    ///     - Create a receipt envelope for the transaction.
    /// 4. Merge all state transitions into the cache state.
    /// 5. Compute the [state root, transactions root, receipts root, logs bloom] for the processed
    ///    block.
    pub fn execute_payload(&mut self, payload: L2PayloadAttributes) -> Result<&Header> {
        // Prepare the `revm` environment.
        let initialized_block_env = Self::prepare_block_env(
            self.revm_spec_id(payload.timestamp),
            self.config.as_ref(),
            &self.parent_header,
            &payload,
        );
        let initialized_cfg = self.evm_cfg_env(payload.timestamp);
        let block_number = initialized_block_env.number.to::<u64>();
        let base_fee = initialized_block_env.basefee.to::<u128>();

        // Apply the pre-block EIP-4788 contract call.
        pre_block_beacon_root_contract_call(
            &mut self.state,
            self.config.as_ref(),
            block_number,
            &initialized_cfg,
            &initialized_block_env,
            &payload,
        )?;

        // Ensure that the create2 contract is deployed upon transition to the Canyon hardfork.
        ensure_create2_deployer_canyon(&mut self.state, self.config.as_ref(), payload.timestamp)?;

        // Construct the EVM with the given configuration.
        // TODO(clabby): Accelerate precompiles w/ custom precompile handler.
        let mut evm = Evm::builder()
            .with_db(&mut self.state)
            .with_env_with_handler_cfg(EnvWithHandlerCfg::new_with_cfg_env(
                initialized_cfg.clone(),
                initialized_block_env.clone(),
                Default::default(),
            ))
            .build();
        let mut cumulative_gas_used = 0u64;
        let mut receipts: Vec<OpReceiptEnvelope> = Vec::with_capacity(payload.transactions.len());

        // Execute the transactions in the payload.
        let transactions = payload
            .transactions
            .iter()
            .map(|raw_tx| {
                let tx = OpTxEnvelope::decode_2718(&mut raw_tx.as_ref()).map_err(|e| anyhow!(e))?;
                Ok((tx, raw_tx.as_ref()))
            })
            .collect::<Result<Vec<_>>>()?;
        for (transaction, raw_transaction) in transactions {
            // Reject any EIP-4844 transactions.
            if matches!(transaction, OpTxEnvelope::Eip4844(_)) {
                anyhow::bail!("EIP-4844 transactions are not supported");
            }

            // If the transaction is a deposit, cache the depositor account.
            //
            // This only needs to be done post-Regolith, as deposit nonces were not included in
            // Bedrock. In addition, non-deposit transactions do not have deposit
            // nonces.
            let depositor = self
                .config
                .is_regolith_active(payload.timestamp)
                .then(|| {
                    if let OpTxEnvelope::Deposit(deposit) = &transaction {
                        evm.db_mut().load_cache_account(deposit.from).ok().cloned()
                    } else {
                        None
                    }
                })
                .flatten();

            // Modify the transaction environment with the transaction data.
            evm = evm
                .modify()
                .modify_tx_env(|tx| {
                    Self::prepare_tx_env(&transaction, raw_transaction, tx)
                        .expect("Failed to prepare tx env")
                })
                .build();

            // Execute the transaction.
            let result = evm.transact_commit().map_err(|e| anyhow!("Fatal EVM Error: {e}"))?;

            // Accumulate the gas used by the transaction.
            cumulative_gas_used += result.gas_used();

            // Create receipt envelope.
            let receipt_envelope = wrap_receipt_with_bloom(
                OpReceiptWithBloom {
                    receipt: OpReceipt {
                        status: result.is_success(),
                        cumulative_gas_used: cumulative_gas_used as u128,
                        logs: result.into_logs(),
                        deposit_nonce: depositor
                            .map(|depositor| depositor.account_info().unwrap_or_default().nonce),
                        // The deposit receipt version was introduced in Canyon to indicate an
                        // update to how receipt hashes should be computed
                        // when set. The state transition process
                        // ensures this is only set for post-Canyon deposit transactions.
                        deposit_receipt_version: self
                            .config
                            .is_canyon_active(payload.timestamp)
                            .then_some(1),
                    },
                    logs_bloom: Default::default(),
                },
                transaction.tx_type(),
            );
            receipts.push(receipt_envelope);
        }

        // Drop the exclusive reference to the state in the EVM so that it may be mutated again.
        drop(evm);

        // Merge all state transitions into the cache state.
        self.state.merge_transitions(BundleRetention::PlainState);

        // Take the bundle state.
        let bundle = self.state.take_bundle();

        // Recompute the header roots.
        let state_root = self.state.database.state_root(&bundle)?;
        let transactions_root = Self::compute_transactions_root(payload.transactions.as_slice());
        let receipts_root =
            Self::compute_receipts_root(&receipts, self.config.as_ref(), payload.timestamp);

        // The withdrawals root on OP Stack chains, after Canyon activation, is always the empty
        // root hash.
        let withdrawals_root =
            self.config.is_canyon_active(payload.timestamp).then_some(EMPTY_ROOT_HASH);

        // Compute logs bloom filter for the block.
        let logs_bloom = logs_bloom(receipts.iter().flat_map(|receipt| receipt.logs()));

        // Compute Cancun fields, if active.
        let (blob_gas_used, excess_blob_gas) = self
            .config
            .is_ecotone_active(payload.timestamp)
            .then(|| {
                let excess_blob_gas = if self.config.is_ecotone_active(self.parent_header.timestamp)
                {
                    let parent_excess_blob_gas =
                        self.parent_header.excess_blob_gas.unwrap_or_default();
                    let parent_blob_gas_used = self.parent_header.blob_gas_used.unwrap_or_default();
                    calc_excess_blob_gas(parent_excess_blob_gas as u64, parent_blob_gas_used as u64)
                } else {
                    // For the first post-fork block, both blob gas fields are evaluated to 0.
                    calc_excess_blob_gas(0, 0)
                };

                (Some(0), Some(excess_blob_gas as u128))
            })
            .unwrap_or_default();

        // Construct the new header.
        let header = Header {
            parent_hash: self.parent_header.seal(),
            ommers_hash: EMPTY_OMMER_ROOT_HASH,
            beneficiary: payload.fee_recipient,
            state_root,
            transactions_root,
            receipts_root,
            withdrawals_root,
            logs_bloom,
            difficulty: U256::ZERO,
            number: block_number,
            gas_limit: payload
                .gas_limit
                .ok_or(anyhow!("Gas limit not provided in payload attributes"))?
                as u128,
            gas_used: cumulative_gas_used as u128,
            timestamp: payload.timestamp,
            mix_hash: payload.prev_randao,
            nonce: Default::default(),
            base_fee_per_gas: Some(base_fee),
            blob_gas_used,
            excess_blob_gas,
            parent_beacon_block_root: payload.parent_beacon_block_root,
            // Provide no extra data on OP Stack chains
            extra_data: Bytes::default(),
        }
        .seal_slow();

        // Update the parent block hash in the state database.
        self.state.database.set_parent_block_hash(header.seal());

        // Update the parent header in the executor.
        self.parent_header = header;

        Ok(&self.parent_header)
    }

    /// Returns the active [SpecId] for the executor.
    ///
    /// ## Takes
    /// - `timestamp`: The timestamp of the executing block.
    ///
    /// ## Returns
    /// The active [SpecId] for the executor.
    fn revm_spec_id(&self, timestamp: u64) -> SpecId {
        if self.config.is_fjord_active(timestamp) {
            // TODO(clabby): Replace w/ Fjord Spec ID, once in a revm release.
            SpecId::ECOTONE
        } else if self.config.is_ecotone_active(timestamp) {
            SpecId::ECOTONE
        } else if self.config.is_canyon_active(timestamp) {
            SpecId::CANYON
        } else if self.config.is_regolith_active(timestamp) {
            SpecId::REGOLITH
        } else {
            SpecId::BEDROCK
        }
    }

    /// Returns the active [CfgEnvWithHandlerCfg] for the executor.
    ///
    /// ## Takes
    /// - `timestamp`: The timestamp of the executing block.
    ///
    /// ## Returns
    /// The active [CfgEnvWithHandlerCfg] for the executor.
    fn evm_cfg_env(&self, timestamp: u64) -> CfgEnvWithHandlerCfg {
        let cfg_env = CfgEnv::default().with_chain_id(self.config.l2_chain_id);
        let mut cfg_handler_env =
            CfgEnvWithHandlerCfg::new_with_spec_id(cfg_env, self.revm_spec_id(timestamp));
        cfg_handler_env.enable_optimism();
        cfg_handler_env
    }

    /// Computes the receipts root from the given set of receipts.
    ///
    /// ## Takes
    /// - `receipts`: The receipts to compute the root for.
    /// - `config`: The rollup config to use for the computation.
    /// - `timestamp`: The timestamp to use for the computation.
    ///
    /// ## Returns
    /// The computed receipts root.
    fn compute_receipts_root(
        receipts: &[OpReceiptEnvelope],
        config: &RollupConfig,
        timestamp: u64,
    ) -> B256 {
        // There is a minor bug in op-geth and op-erigon where in the Regolith hardfork,
        // the receipt root calculation does not inclide the deposit nonce in the
        // receipt encoding. In the Regolith hardfork, we must strip the deposit nonce
        // from the receipt encoding to match the receipt root calculation.
        if config.is_regolith_active(timestamp) && !config.is_canyon_active(timestamp) {
            let receipts = receipts
                .iter()
                .cloned()
                .map(|receipt| match receipt {
                    OpReceiptEnvelope::Deposit(mut deposit_receipt) => {
                        deposit_receipt.receipt.deposit_nonce = None;
                        OpReceiptEnvelope::Deposit(deposit_receipt)
                    }
                    _ => receipt,
                })
                .collect::<Vec<_>>();

            ordered_trie_with_encoder(receipts.as_ref(), |receipt, mut buf| {
                receipt.encode_2718(&mut buf)
            })
            .root()
        } else {
            ordered_trie_with_encoder(receipts, |receipt, mut buf| receipt.encode_2718(&mut buf))
                .root()
        }
    }

    /// Computes the transactions root from the given set of encoded transactions.
    ///
    /// ## Takes
    /// - `transactions`: The transactions to compute the root for.
    ///
    /// ## Returns
    /// The computed transactions root.
    fn compute_transactions_root(transactions: &[RawTransaction]) -> B256 {
        ordered_trie_with_encoder(transactions, |tx, buf| buf.put_slice(tx.as_ref())).root()
    }

    /// Prepares a [BlockEnv] with the given [L2PayloadAttributes].
    ///
    /// ## Takes
    /// - `payload`: The payload to prepare the environment for.
    /// - `env`: The block environment to prepare.
    fn prepare_block_env(
        spec_id: SpecId,
        config: &RollupConfig,
        parent_header: &Header,
        payload_attrs: &L2PayloadAttributes,
    ) -> BlockEnv {
        let blob_excess_gas_and_price = parent_header
            .next_block_excess_blob_gas()
            .or_else(|| spec_id.is_enabled_in(SpecId::ECOTONE).then_some(0))
            .map(|x| BlobExcessGasAndPrice::new(x as u64));
        let next_block_base_fee = parent_header
            .next_block_base_fee(config.base_fee_params_at_timestamp(payload_attrs.timestamp))
            .unwrap_or_default();

        BlockEnv {
            number: U256::from(parent_header.number + 1),
            coinbase: address!("4200000000000000000000000000000000000011"),
            timestamp: U256::from(payload_attrs.timestamp),
            gas_limit: U256::from(payload_attrs.gas_limit.expect("Gas limit not provided")),
            basefee: U256::from(next_block_base_fee),
            difficulty: U256::ZERO,
            prevrandao: Some(payload_attrs.prev_randao),
            blob_excess_gas_and_price,
        }
    }

    /// Prepares a [TxEnv] with the given [OpTxEnvelope].
    ///
    /// ## Takes
    /// - `transaction`: The transaction to prepare the environment for.
    /// - `env`: The transaction environment to prepare.
    ///
    /// ## Returns
    /// - `Ok(())` if the environment was successfully prepared.
    /// - `Err(_)` if an error occurred while preparing the environment.
    fn prepare_tx_env(
        transaction: &OpTxEnvelope,
        encoded_transaction: &[u8],
        env: &mut TxEnv,
    ) -> Result<()> {
        match transaction {
            OpTxEnvelope::Legacy(signed_tx) => {
                let tx = signed_tx.tx();
                env.caller = signed_tx
                    .recover_signer()
                    .map_err(|e| anyhow!("Failed to recover signer: {}", e))?;
                env.gas_limit = tx.gas_limit as u64;
                env.gas_price = U256::from(tx.gas_price);
                env.gas_priority_fee = None;
                env.transact_to = match tx.to {
                    TxKind::Call(to) => TransactTo::Call(to),
                    TxKind::Create => TransactTo::create(),
                };
                env.value = tx.value;
                env.data = tx.input.clone();
                env.chain_id = tx.chain_id;
                env.nonce = Some(tx.nonce);
                env.access_list.clear();
                env.blob_hashes.clear();
                env.max_fee_per_blob_gas.take();
                env.optimism = OptimismFields {
                    source_hash: None,
                    mint: None,
                    is_system_transaction: Some(false),
                    enveloped_tx: Some(encoded_transaction.to_vec().into()),
                };
                Ok(())
            }
            OpTxEnvelope::Eip2930(signed_tx) => {
                let tx = signed_tx.tx();
                env.caller = signed_tx
                    .recover_signer()
                    .map_err(|e| anyhow!("Failed to recover signer: {}", e))?;
                env.gas_limit = tx.gas_limit as u64;
                env.gas_price = U256::from(tx.gas_price);
                env.gas_priority_fee = None;
                env.transact_to = match tx.to {
                    TxKind::Call(to) => TransactTo::Call(to),
                    TxKind::Create => TransactTo::create(),
                };
                env.value = tx.value;
                env.data = tx.input.clone();
                env.chain_id = Some(tx.chain_id);
                env.nonce = Some(tx.nonce);
                env.access_list = tx
                    .access_list
                    .0
                    .iter()
                    .map(|l| {
                        (
                            l.address,
                            l.storage_keys.iter().map(|k| U256::from_be_bytes(k.0)).collect(),
                        )
                    })
                    .collect();
                env.blob_hashes.clear();
                env.max_fee_per_blob_gas.take();
                env.optimism = OptimismFields {
                    source_hash: None,
                    mint: None,
                    is_system_transaction: Some(false),
                    enveloped_tx: Some(encoded_transaction.to_vec().into()),
                };
                Ok(())
            }
            OpTxEnvelope::Eip1559(signed_tx) => {
                let tx = signed_tx.tx();
                env.caller = signed_tx
                    .recover_signer()
                    .map_err(|e| anyhow!("Failed to recover signer: {}", e))?;
                env.gas_limit = tx.gas_limit as u64;
                env.gas_price = U256::from(tx.max_fee_per_gas);
                env.gas_priority_fee = Some(U256::from(tx.max_priority_fee_per_gas));
                env.transact_to = match tx.to {
                    TxKind::Call(to) => TransactTo::Call(to),
                    TxKind::Create => TransactTo::create(),
                };
                env.value = tx.value;
                env.data = tx.input.clone();
                env.chain_id = Some(tx.chain_id);
                env.nonce = Some(tx.nonce);
                env.access_list = tx
                    .access_list
                    .0
                    .iter()
                    .map(|l| {
                        (
                            l.address,
                            l.storage_keys.iter().map(|k| U256::from_be_bytes(k.0)).collect(),
                        )
                    })
                    .collect();
                env.blob_hashes.clear();
                env.max_fee_per_blob_gas.take();
                env.optimism = OptimismFields {
                    source_hash: None,
                    mint: None,
                    is_system_transaction: Some(false),
                    enveloped_tx: Some(encoded_transaction.to_vec().into()),
                };
                Ok(())
            }
            OpTxEnvelope::Deposit(tx) => {
                env.caller = tx.from;
                env.access_list.clear();
                env.gas_limit = tx.gas_limit as u64;
                env.gas_price = U256::ZERO;
                env.gas_priority_fee = None;
                match tx.to {
                    TxKind::Call(to) => env.transact_to = TransactTo::Call(to),
                    TxKind::Create => env.transact_to = TransactTo::create(),
                }
                env.value = tx.value;
                env.data = tx.input.clone();
                env.chain_id = None;
                env.nonce = None;
                env.optimism = OptimismFields {
                    source_hash: Some(tx.source_hash),
                    mint: tx.mint,
                    is_system_transaction: Some(tx.is_system_transaction),
                    enveloped_tx: Some(encoded_transaction.to_vec().into()),
                };
                Ok(())
            }
            _ => anyhow::bail!("Unexpected tx type"),
        }
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use std::format;

    use super::*;
    use alloc::string::{String, ToString};
    use alloy_primitives::{address, b256, hex};
    use alloy_rlp::Decodable;

    /// A [TrieDBFetcher] implementation that fetches trie nodes and bytecode from the local
    /// testdata folder.
    struct TestdataTrieDBFetcher {
        testdata_folder: String,
    }

    impl TrieDBFetcher for TestdataTrieDBFetcher {
        fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
            let file_name = format!("testdata/{}/{}.bin", self.testdata_folder, hex::encode(key));
            std::fs::read(&file_name)
                .map_err(|e| anyhow!("Failed to read {file_name}: {}", e))
                .map(Into::into)
        }

        fn bytecode_by_hash(&self, code_hash: B256) -> Result<Bytes> {
            let file_name =
                format!("testdata/{}/{}.bin", self.testdata_folder, hex::encode(code_hash));
            std::fs::read(&file_name)
                .map_err(|e| anyhow!("Failed to read {file_name}: {}", e))
                .map(Into::into)
        }

        fn header_by_hash(&self, hash: B256) -> Result<Header> {
            let file_name = format!("testdata/{}/{}.bin", self.testdata_folder, hex::encode(hash));
            let encoded_header = std::fs::read(&file_name)
                .map_err(|e| anyhow!("Failed to read {file_name}: {}", e))?;
            Header::decode(&mut encoded_header.as_slice()).map_err(|e| anyhow!(e))
        }
    }

    #[test]
    fn test_l2_block_executor_small_block() {
        // Static for the execution of block #120794432 on OP mainnet.
        // https://optimistic.etherscan.io/block/120794432

        // Make a mock rollup config, with Ecotone activated at timestamp = 0.
        let rollup_config = RollupConfig {
            l2_chain_id: 10,
            regolith_time: Some(0),
            canyon_time: Some(0),
            delta_time: Some(0),
            ecotone_time: Some(0),
            ..Default::default()
        };

        // Decode the headers.
        let raw_header = hex!("f90244a0ff7c6abc94edcaddd02c12ec7d85ffbb3ba293f3b76897e4adece57e692bcc39a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a0a0b24abb13d6149947247a8817517971bb8d213de1e23225e2b20d36a5b6427ca0c31e4a2ada52ac698643357ca89ef2740d384076ef0e17b653bcb6ea7dd8902ea09f4fcf34e78afc216240e3faa72c822f8eea4757932eb9e0fd42839d192bb903b901000440000210068007000000940000000220000006000820048404800002000004040100001b2000008800001040000018280000400001200004000101086000000802800080004008010001080000200100a00000204840000118042080000400804001000a0400080200111000000800050000020200064000000012000800048000000000101800200002000000080008001581402002200210341089000080c2d004106000000018000000804285800800000020000180008000020000000000020103410400000000200400008000280400000100020000002002000021000811000920808000010000000200210400000020008000400000000000211008808407332d3f8401c9c3808327c44d84665a343780a0edba75784acf3165bffd96df8b78ffdb3781db91f886f22b4bee0a6f722df93988000000000000000083202ef8a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a0917693152c4a041efbc196e9d169087093336da96a8bb3af1e55fce447a7b8a9");
        let header = Header::decode(&mut &raw_header[..]).unwrap();
        let raw_expected_header = hex!("f90243a09506905902f5c3613c5441a8697c09e7aafdb64082924d8bd2857f9e34a47a9aa01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a0a1e9207c3c68cd4854074f08226a3643debed27e45bf1b22ab528f8de16245eda0121e8765953af84974b845fd9b01f5ff9b0f7d2886a2464535e8e9976a1c8daba092c6a5e34d7296d63d1698258c40539a20080c668fc9d63332363cfbdfa37976b9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000808407332d408401c9c38082ab4b84665a343980a0edba75784acf3165bffd96df8b78ffdb3781db91f886f22b4bee0a6f722df93988000000000000000083201f31a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a0917693152c4a041efbc196e9d169087093336da96a8bb3af1e55fce447a7b8a9");
        let expected_header = Header::decode(&mut &raw_expected_header[..]).unwrap();

        // Initialize the block executor on block #120794431's post-state.
        let mut l2_block_executor = StatelessL2BlockExecutor::new(
            Arc::new(rollup_config),
            b256!("a0b24abb13d6149947247a8817517971bb8d213de1e23225e2b20d36a5b6427c"),
            header.seal_slow(),
            TestdataTrieDBFetcher { testdata_folder: "block_120794432_exec".to_string() },
        );

        let raw_tx = hex!("7ef8f8a003b511b9b71520cd62cad3b5fd5b1b8eaebd658447723c31c7f1eba87cfe98c894deaddeaddeaddeaddeaddeaddeaddeaddead00019442000000000000000000000000000000000000158080830f424080b8a4440a5e2000000558000c5fc5000000000000000300000000665a33a70000000001310e960000000000000000000000000000000000000000000000000000000214d2697300000000000000000000000000000000000000000000000000000000000000015346d208a396843018a2e666c8e7832067358433fb87ca421273c6a4e69f78d50000000000000000000000006887246668a3b87f54deb3b94ba47a6f63f32985");
        let payload_attrs = L2PayloadAttributes {
            fee_recipient: address!("4200000000000000000000000000000000000011"),
            gas_limit: Some(0x1c9c380),
            timestamp: 0x665a3439,
            prev_randao: b256!("edba75784acf3165bffd96df8b78ffdb3781db91f886f22b4bee0a6f722df939"),
            withdrawals: Default::default(),
            parent_beacon_block_root: Some(b256!(
                "917693152c4a041efbc196e9d169087093336da96a8bb3af1e55fce447a7b8a9"
            )),
            transactions: alloc::vec![raw_tx.into()],
            no_tx_pool: false,
        };
        let produced_header = l2_block_executor.execute_payload(payload_attrs).unwrap().clone();

        assert_eq!(produced_header, expected_header);
        assert_eq!(l2_block_executor.parent_header.seal(), expected_header.hash_slow());
        std::println!(
            "Hash matches! {} == {}",
            hex::encode(produced_header.hash_slow()),
            hex::encode(expected_header.hash_slow())
        );
    }
}
