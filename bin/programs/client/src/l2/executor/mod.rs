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
    pub fn new(config: Arc<RollupConfig>, parent_header: Sealed<Header>, fetcher: F) -> Self {
        let trie_db = TrieDB::new(parent_header.state_root, parent_header.seal(), fetcher);
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
        let gas_limit =
            payload.gas_limit.ok_or(anyhow!("Gas limit not provided in payload attributes"))?;

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
        // TODO(clabby): Pass parent header
        ensure_create2_deployer_canyon(&mut self.state, self.config.as_ref(), payload.timestamp)?;

        // Construct the EVM with the given configuration.
        // TODO(clabby): Accelerate precompiles w/ custom precompile handler.
        let mut cumulative_gas_used = 0u64;
        let mut receipts: Vec<OpReceiptEnvelope> = Vec::with_capacity(payload.transactions.len());
        let is_regolith = self.config.is_regolith_active(payload.timestamp);

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
            // The sum of the transaction’s gas limit, Tg, and the gas utilized in this block prior,
            // must be no greater than the block’s gasLimit.
            // let block_available_gas = gas_limit - cumulative_gas_used;
            // if transaction.gas_limit() > block_available_gas
            //     && (is_regolith || !transaction.is_system_transaction())
            // {
            //     anyhow::bail!("Transaction gas limit exceeds block gas limit")
            // }

            // Reject any EIP-4844 transactions.
            if matches!(transaction, OpTxEnvelope::Eip4844(_)) {
                anyhow::bail!("EIP-4844 transactions are not supported");
            }

            let mut evm = Evm::builder()
                .with_db(&mut self.state)
                .with_env_with_handler_cfg(EnvWithHandlerCfg::new_with_cfg_env(
                    initialized_cfg.clone(),
                    initialized_block_env.clone(),
                    Self::prepare_tx_env(&transaction, raw_transaction)?,
                ))
                .build();

            // If the transaction is a deposit, cache the depositor account.
            //
            // This only needs to be done post-Regolith, as deposit nonces were not included in
            // Bedrock. In addition, non-deposit transactions do not have deposit
            // nonces.
            let depositor = is_regolith
                .then(|| {
                    if let OpTxEnvelope::Deposit(deposit) = &transaction {
                        evm.db_mut().load_cache_account(deposit.from).ok().cloned()
                    } else {
                        None
                    }
                })
                .flatten();

            // Execute the transaction.
            let result = evm.transact_commit().map_err(|e| anyhow!("Fatal EVM Error: {e}"))?;

            // Accumulate the gas used by the transaction.
            cumulative_gas_used += result.gas_used();

            // Create receipt envelope.
            let logs_bloom = logs_bloom(result.logs());
            let receipt_envelope = wrap_receipt_with_bloom(
                OpReceiptWithBloom {
                    receipt: OpReceipt {
                        status: result.is_success(),
                        cumulative_gas_used: cumulative_gas_used as u128,
                        logs: result.into_logs(),
                        deposit_nonce: depositor
                            .as_ref()
                            .map(|depositor| depositor.account_info().unwrap_or_default().nonce),
                        // The deposit receipt version was introduced in Canyon to indicate an
                        // update to how receipt hashes should be computed
                        // when set. The state transition process
                        // ensures this is only set for post-Canyon deposit transactions.
                        deposit_receipt_version: depositor
                            .is_some()
                            .then(|| self.config.is_canyon_active(payload.timestamp).then_some(1))
                            .flatten(),
                    },
                    logs_bloom,
                },
                transaction.tx_type(),
            );
            receipts.push(receipt_envelope);
        }

        // Merge all state transitions into the cache state.
        self.state.merge_transitions(BundleRetention::Reverts);

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
            gas_limit: gas_limit.into(),
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
    fn prepare_tx_env(transaction: &OpTxEnvelope, encoded_transaction: &[u8]) -> Result<TxEnv> {
        let mut env = TxEnv::default();
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
                Ok(env)
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
                Ok(env)
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
                Ok(env)
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
                Ok(env)
            }
            _ => anyhow::bail!("Unexpected tx type"),
        }
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use std::{collections::HashMap, format};

    use super::*;
    use alloc::string::{String, ToString};
    use alloy_primitives::{address, b256, hex};
    use alloy_rlp::Decodable;
    use serde::Deserialize;

    /// A [TrieDBFetcher] implementation that fetches trie nodes and bytecode from the local
    /// testdata folder.
    #[derive(Deserialize)]
    struct TestdataTrieDBFetcher {
        preimages: HashMap<B256, Bytes>,
    }

    impl TestdataTrieDBFetcher {
        /// Constructs a new [TestdataTrieDBFetcher] with the given testdata folder.
        pub fn new(testdata_folder: &str) -> Self {
            let file_name = format!("testdata/{}/output.json", testdata_folder);
            let preimages = serde_json::from_str::<HashMap<B256, Bytes>>(
                &std::fs::read_to_string(&file_name).unwrap(),
            )
            .unwrap();
            Self { preimages }
        }
    }

    impl TrieDBFetcher for TestdataTrieDBFetcher {
        fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
            self.preimages
                .get(&key)
                .map(|v| v.clone())
                .ok_or_else(|| anyhow!("Preimage not found for key: {}", key))
        }

        fn bytecode_by_hash(&self, code_hash: B256) -> Result<Bytes> {
            self.preimages
                .get(&code_hash)
                .map(|v| v.clone())
                .ok_or_else(|| anyhow!("Bytecode not found for hash: {}", code_hash))
        }

        fn header_by_hash(&self, hash: B256) -> Result<Header> {
            let encoded_header = self
                .preimages
                .get(&hash)
                .ok_or_else(|| anyhow!("Header not found for hash: {}", hash))?;
            Header::decode(&mut encoded_header.as_ref()).map_err(|e| anyhow!(e))
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
            header.seal_slow(),
            TestdataTrieDBFetcher::new("block_120794432_exec"),
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
    }

    #[test]
    fn test_l2_block_executor_small_block_2() {
        // Static for the execution of block #121049889 on OP mainnet.
        // https://optimistic.etherscan.io/block/121049889

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
        let raw_parent_header = hex!("f90245a0311e3aa67dca0d157b8e8a4e117a4fd34cedcebc63f5708976e86581c07824a5a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a0b1772b8cd400c2d2cfee5bd294bcc399e4c8330d856907f95d2305a64ff9c968a0a42b2ec1d1e928f2b63224888d482f72537ee392e98390c760c902ca3f7d75d8a0e993b3cac72163177e7e728c5e4d074551b181a45f49b0026c48e893f7b4768eb901008008140067b0392a00048280488c10a04000180084400038834008020400c960003c9000068083b00000f00cc40088ab48306c402008068f0810881b84342000860104c10500102b209410584214804a40034000080d622018042ca008000204a016089206020412050c1902440158505802207070800900020028facaacc0101e0a08000010a003a15166a231024090841918038500ac4082281880810648221200881000116002c0444044421024c6c401c0008d42280c98408085142c3041542272832790b4154e66c082080a2090100002409548047010c208220588622694900120454200800600104100e01a160214408c4000141890022802209102488084073713208401c9c380831f42d1846661fff980a02ea5360883566f7bf998c6ce46367b64aeb24c0178a6e5752ea796ca9b9f951988000000000000000084038e4654a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a0025cfb4d23d2384982b73c2669eeb4fb73b29960750554e2380af54add10dbda");
        let parent_header = Header::decode(&mut &raw_parent_header[..]).unwrap();
        let raw_expected_header = hex!("f90245a0925b8e3c7216dd1c62e3fd9911f6cb3f456b9aa685f34239180d1a7ef7653b7fa01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a0ac6f1a9722101300ba71fb58517eadbb4964dc4f4891f8f3e58a292e7c3204f3a032ae1c22601d63eaa26aa5ab30e6b8ae1cdfb7104c0067327d91bc3094461fc9a016c68c81160c03fa72763fdd578c6a5563cca47ded1a54df3610c0412b976b25b90100000004000000000000000000001000000001200000000000000000000040000000001000200000000000000000000000200000000000000001000000000420000200000200002000000000800000000000000000000400000000000000012000000000000200000040008400050009000000000000000000000000000200050000000000000000000000010000000000000050840000000000000000000010000000000400000000000000000000008000000000010000000000000000000804000000000008000001000010000000000000840000080000100000000000600000000000000000002100000000000000001000000000008000000800000000008084073713218401c9c380830505a2846661fffb80a0d91ae18a8b94471ef1b15686ef8a6144a109b837c28488a0f1a2a4e4ad29d5af88000000000000000084038c2024a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a05e7da14ac6b18e62306c84d9d555387d4b4a6c3d122df22a2af2b68bf219860d");
        let expected_header = Header::decode(&mut &raw_expected_header[..]).unwrap();

        // Initialize the block executor on block #121049888's post-state.
        let mut l2_block_executor = StatelessL2BlockExecutor::new(
            Arc::new(rollup_config),
            parent_header.seal_slow(),
            TestdataTrieDBFetcher::new("block_121049889_exec"),
        );

        let raw_txs = alloc::vec![
            hex!("7ef8f8a01e6036fa5dc5d76e0095f42fef2c4aa7d6589b4f496f9c4bea53daef1b4a24c194deaddeaddeaddeaddeaddeaddeaddeaddead00019442000000000000000000000000000000000000158080830f424080b8a4440a5e2000000558000c5fc50000000000000000000000006661ff73000000000131b40700000000000000000000000000000000000000000000000000000005c9ea450a0000000000000000000000000000000000000000000000000000000000000001e885b088376fedbd0490a7991be47854872f6467c476d255eed3151d5f6a95940000000000000000000000006887246668a3b87f54deb3b94ba47a6f63f32985").into(),
            hex!("02f9010b0a8301b419835009eb840439574783030fc3940000000000002bdbf1bf3279983603ec279cc6df8702c2ad68fd9000b89666e0daa0001e80001ec0001d0220001e01001e01000bfe1561df392590b0cb3ec093b711066774ca96cd001e01001e20001ee49dbb844d000b3678862f04290e565cca2ef163baeb92bb76790c001e01001e01001ea0000b38873c13509d36077a4638183f4a9a72f8a66b91001e20000bcaaef30cf6e70a0118e59cd3fb88164de6d144b5003a01001802c2ad68fd900000012b817fc001a098c44ee6585f33a4fbc9c999b2469697dd8007b986c79569ae6f3d077de45a1ca035c3ea5e954ae76fdf75f7d7ce215a339ac20a772081b62908d5fcf551693e3a").into(),
            hex!("02f904920a828a19834c4b408403dce3e7837a1200944d75a5ce454b264b187bee9e189af1564a68408d80b90424b1dc65a400018958e0d17c70a7bddf525ee0a3bf00f5c8f886a03156c522c0b256cb884d00000000000000000000000000000000000000000000000000000000001814035a6bc28056dae2cfa8a6479f5e50eee95bb3ae2b65be853a4440f15cb60211ba00000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000026000000000000000000000000000000000000000000000000000000000000003400000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000b2c639c533813f4aa9d7837caf62653d097ff85000000000000000000000000000000000000000000000000000000e8d4a510000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000606ecf709c09afd92138cca6ee144be81e1c6ef231d4586a22eb7fc801826e837691e208839c1c58d50a31826c8b47c5218c3898ee6671f734bd9b9584ce210e8b1fb287f374f07a99bbce2ddedc655ee5c94f8fee715db21644ae134638af8c32d18b1d27dbc2e12b205ea25ab6bb4ec447ee7f40dba560e511a20fd8a3775d04ad83bf593e3587be1dd85ab9b2053d1386fae00c5fdea56a68ea147b706e5ced65ab296b8d9248aa943787a5c8aa4fd56ba7133d087e84a625fe1c3d8a390b5000000000000000000000000000000000000000000000000000000000000000666634013473fce9d0696d9f0375be4260a81518a85f2482b3f5336848f8fa3ce1a3f7032124577ee2a755122f916e4fe757fc42eb5561216892ed806d368908b69c4d4d1cd06897a3a2f02c17ffba7a762e4cbbdb086a1181f1111874f88f38f3b86fa03508822346a167de3f6afc9066cc274103cf18d62c7d6a4d93dcd000b7842951fd9a14a647148dac543f446cd9427dedbc3c3ca5ed2b36f5c27ce76de46d4291be6ef3b41679501c8f0341d35cf6afc9f7d91d56ad1a8ae34fc0e708ac001a013f549ca84754e18fae518daa617d19dfbdff6da7bc794bab89e7a17288cb5b5a00c4913669beb11412e9e04bd4311ed5b11443b9e34f7fb25488e58047ddd8820").into()
        ];
        let payload_attrs = L2PayloadAttributes {
            fee_recipient: address!("4200000000000000000000000000000000000011"),
            gas_limit: Some(30000000),
            timestamp: 1717698555,
            prev_randao: b256!("d91ae18a8b94471ef1b15686ef8a6144a109b837c28488a0f1a2a4e4ad29d5af"),
            withdrawals: Default::default(),
            parent_beacon_block_root: Some(b256!(
                "5e7da14ac6b18e62306c84d9d555387d4b4a6c3d122df22a2af2b68bf219860d"
            )),
            transactions: raw_txs,
            no_tx_pool: false,
        };
        let produced_header = l2_block_executor.execute_payload(payload_attrs).unwrap().clone();

        assert_eq!(produced_header, expected_header);
        assert_eq!(l2_block_executor.parent_header.seal(), expected_header.hash_slow());
    }

    #[test]
    fn test_l2_block_executor_small_block_3() {
        // Static for the execution of block #121003241 on OP mainnet.
        // https://optimistic.etherscan.io/block/121003241

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
        let raw_parent_header = hex!("f90245a01fe9a4a3f3a03b5e9bf26739dc0402016bcd0b4eba84f6daec89cd25ede03785a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a0f0f4294d35c59be9ac60e3c8b10f72f082eb20db04e84b89622eaf36dc288f94a037567276c3663d85aa9c8f6d9fa3a9b02511a5314c08d83648caae01da377f0da0a5cc7888ada10b0cf445632d9239c129cb55b9822edcc6062262660cc9786457b9010007000032410480052001888000000000000200000400200040040000442002000a892000100000020008001100112000000000408000b012000002c200b48080000068040001480885003408000880010044000010241440800428208400004044000880820800800100100000000801820000000000000081000030000800204000000840000000802a0000000100400004000180300000004120104000001922000102000000000060001289c024840010000521800000000022140000208040001203800420620019020200004000209008009000000000004000880070120010220820502000500400202000000000040028000089c00080100000010008808407365ce88401c9c380832415e9846660938980a022e77867678dc60aace7567ee344620f47a66be343eac90a82bf619ea37de357880000000000000000840398f69aa056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a050f4a35e2f059621cba649e719d23a2a9d030189fd19172a689c76d3adf39fec");
        let parent_header = Header::decode(&mut &raw_parent_header[..]).unwrap();
        let raw_expected_header = hex!("f90245a090957c484fec69a6b308f18d83a320b18a5471ba9566e5b56dfc656abd354744a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a049dfddc9ce6d832c6ab981aea324c3d57b1b1d93823656b43d02608e6b59f3bda0533a1c4f39fa301e354292186123681d97ae64a788cf2af61e6f70e3080c1ac3a0c888d1dfb9590590036630c91d4ff2401a4946524f315bffbbbed795820e3744b90100060000024200002000118880000000008004000104000000000000000400010000080000000000000000040100000000000800c08000200a0000020000200080000000040040000800000008000000000040080004000000804000010002000040802088028c0010000014000200080102001000000800000000001000082000000000002000000000000000000000000044100080200000000100000c00800002000040001100000040100280000400040480000000000000800600000020c040001402008000401001201620020000000000000004000000800200000320000010200200080000400000000000040000000004008080002000000000010000808407365ce98401c9c3808312f8db846660938b80a022e77867678dc60aace7567ee344620f47a66be343eac90a82bf619ea37de3578800000000000000008403970597a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a050f4a35e2f059621cba649e719d23a2a9d030189fd19172a689c76d3adf39fec");
        let expected_header = Header::decode(&mut &raw_expected_header[..]).unwrap();

        // Initialize the block executor on block #121003240's post-state.
        let mut l2_block_executor = StatelessL2BlockExecutor::new(
            Arc::new(rollup_config),
            parent_header.seal_slow(),
            TestdataTrieDBFetcher::new("block_121003241_exec"),
        );

        let raw_txs = alloc::vec![
            hex!("7ef8f8a02c3adbd572915b3ef2fe7c81418461cb32407df8cb1bd4c1f5f4b45e474bfce694deaddeaddeaddeaddeaddeaddeaddeaddead00019442000000000000000000000000000000000000158080830f424080b8a4440a5e2000000558000c5fc5000000000000000400000000666092ff00000000013195d800000000000000000000000000000000000000000000000000000004da0e1101000000000000000000000000000000000000000000000000000000000000000493a1359bf7a89d8b2b2073a153c47f9c399f8f7a864e4f25744d6832cb6fadd80000000000000000000000006887246668a3b87f54deb3b94ba47a6f63f32985").into(),
            hex!("f86a03840998d150827b0c9422fb762f614ede47d33ca2de13a5fb16354a7a5b872defc438f220008038a0e83ca5fd673c57230b1ea308752959568a795fc0b2eccc4128bb295673f4f576a04de60eb10a6aa6fcffd5a956523a92451b06cf669cf332139ac2937880e4ee2f").into(),
            hex!("f87e8301abd284050d2c55830493e094a43305ce0164d87d7b2368f91a1dcc4ebda751278097c201015dc7073aac5a2702007a6c235e4c4f676660938937a07575b3c2ed04981845adc29fc27bf573ccd17462c2d5789e3844d66d29277a79a005175e178a234d48c7e15bfaa979f1b78636228d550a200d9e34e05169d1b770").into(),
            hex!("02f90fb40a83136342840104b33a840836a06e830995ae94087000a300de7200382b55d40045000000e5d60e80b90f4482ad56cb000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000042000000000000000000000000000000000000000000000000000000000000007c00000000000000000000000000000000000000000000000000000000000000b600000000000000000000000008f7dbe4fa3818025d82bb10190f178eaf5992bea0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000003046a761202000000000000000000000000b5fbfeba9848664fd1a49dc2a250d9b5d1294f2a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002800000000000000000000000000000000000000000000000000000000000000104414bf389000000000000000000000000dc6ff44d5d932cbd77b52e5612ba0529dc6226f10000000000000000000000007f5c764cbc14f9669b88837ca1490cca17c3160700000000000000000000000000000000000000000000000000000000000027100000000000000000000000008f7dbe4fa3818025d82bb10190f178eaf5992bea000000000000000000000000000000000000000000000000000000006660a175000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000000000000000000000004a71a1f00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000419a434a72274666c423432aad2ffb19565424d0c6e2d17fc64934b3e4fec97788446afa2d830e2dd926c04ce882e601cb9fa398149b5d778cbe3ebe6038e8643e1b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a34049de917233a7516aa01fc0bad683a6a8b29d0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000003046a761202000000000000000000000000b5fbfeba9848664fd1a49dc2a250d9b5d1294f2a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002800000000000000000000000000000000000000000000000000000000000000104414bf389000000000000000000000000dc6ff44d5d932cbd77b52e5612ba0529dc6226f10000000000000000000000007f5c764cbc14f9669b88837ca1490cca17c316070000000000000000000000000000000000000000000000000000000000002710000000000000000000000000a34049de917233a7516aa01fc0bad683a6a8b29d000000000000000000000000000000000000000000000000000000006660a17b0000000000000000000000000000000000000000000000002870624346de10000000000000000000000000000000000000000000000000000000000000d8ecb600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000418217f8941b74fc2cd49b297652e34ba54465a905ccc5fd452b48fd40a82502590c4c48e64b2a0f0e8e8793a13addfe6d4937bf78d9875a4d9002266be5ecc0a41b0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000fb5049c82e7fa9e7011ddd435b30652b48a1195b0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000003046a761202000000000000000000000000b5fbfeba9848664fd1a49dc2a250d9b5d1294f2a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002800000000000000000000000000000000000000000000000000000000000000104414bf3890000000000000000000000007f5c764cbc14f9669b88837ca1490cca17c31607000000000000000000000000dc6ff44d5d932cbd77b52e5612ba0529dc6226f10000000000000000000000000000000000000000000000000000000000002710000000000000000000000000fb5049c82e7fa9e7011ddd435b30652b48a1195b000000000000000000000000000000000000000000000000000000006660a1890000000000000000000000000000000000000000000000000000000000d019f10000000000000000000000000000000000000000000000002543ff48d0da90eb00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000413698ad34509d153bf3d7287553d81c098983d590f5c9e80c95c361de3c220c745eafd0ca4ef4e78cffe29e7b346ee3d134d20eebd9d98663438646a1ea3801d61c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000009ef549707a5d504c24b0627aff2eb845e8ae02d80000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000003046a761202000000000000000000000000b5fbfeba9848664fd1a49dc2a250d9b5d1294f2a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002800000000000000000000000000000000000000000000000000000000000000104414bf38900000000000000000000000068f180fcce6836688e9084f035309e29bf0a20950000000000000000000000007f5c764cbc14f9669b88837ca1490cca17c3160700000000000000000000000000000000000000000000000000000000000001f40000000000000000000000009ef549707a5d504c24b0627aff2eb845e8ae02d8000000000000000000000000000000000000000000000000000000006660a188000000000000000000000000000000000000000000000000000000000000048200000000000000000000000000000000000000000000000000000000000c78cc0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000041d561852d56b0baac02af7a38ac72d7f560d4a0956032e051adb598fbdb035661280071192a277daf0d36667dc88155f9b445a465dbbadc3149b3ee6c07ae905d1c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c080a0f2c4eec1941db4f698a0fc5b24d708d4231decf19719977bca15af04cbd39cc6a022036042105c9ede61cf13552f6c2d712a3eefbb4f47df3cbe3d3b9b46723398").into(),
            hex!("02f8af0a8083989680840578b8db83025dbe94dc6ff44d5d932cbd77b52e5612ba0529dc6226f180b844a9059cbb00000000000000000000000056c38d1b4676c9c2259d0820dcbce069d3321d5f00000000000000000000000000000000000000000000000029563f7ac07ae000c080a0d0b1d61b918d88059cc8dbee2833c2ce78573b76c731e266d110ed330fb72563a05ca02995f5ec74c0bd9b7209785d75369a1f43a5f045189a51f851ea9b9a791b").into(),
            hex!("02f8740a832c6a52834c4b4085012a05f200825208948c1e1a0b0f9420139e12fa1379b6a76d381d7c8f870a18f74161700080c001a00b7dcc69c346c674167fdd0cee4b13622838d4d9a1f64ef0270d366e61c49fdaa02d99fcd56b7ef8aec6a04c0204a6fd66dcddb755cd54226527a51e5ba22aacd7").into(),
            hex!("f86a808403b23254825208945e809a85aa182a9921edd10a4163745bb3e362848704f7793d6560098038a0c921dce37651444a6c3004e85263d7ef593225d6f5a6ac19265c5a1044f598caa003cbfcc7b3d89a023c7d423496bc0f55c281c501cdd00909e6e09485d90d6500").into(),
            hex!("f8aa8207a88403a9e89182cac994dc6ff44d5d932cbd77b52e5612ba0529dc6226f180b844a9059cbb0000000000000000000000002e2927d05851ae228ab68dd04434dece401cf72b00000000000000000000000000000000000000000000000029998b20cdd0c00038a0a3d6514ad022c5b79f8b41cb59b7e48b62ca90d409a5438783f89947009a548ea037de75cc680392eac97820b5884239ca0a0a990e63fc118b0040b631ac73fc52").into(),
            hex!("02f905720a820a1e830f42408404606a2c83044bc0940000000071727de22e5e9d8baf0edac6f37da03280b90504765e827f00000000000000000000000000000000000000000000000000000000000000400000000000000000000000004337016838785634c63fce393bfc6222564436c4000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000006a2aad2e20ef62b5b56e4e2b5e342e53ee7fa04f000017719c140000000000000000000000000000000005300000000000000002000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000002265a0000000000000000000000000001d4c00000000000000000000000000000000000000000000000000000000000010a370000000000000000000000000010c8e000000000000000000000000004d4157c00000000000000000000000000000000000000000000000000000000000003200000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001a4e9ae5c530100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000006668bc6eea73404b4da5775c774fafc815b66b36000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000044a9059cbb000000000000000000000000efe1bfc13a0f086066fbe23a18c896eb697ca5cc00000000000000000000000000000000000000000000000000000001a13b8600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000b59d0021a869f1ed3a661ffe8c9b41ec6244261d9800000000000000000000000000004e8a0000000000000000000000000000000100000000000000000000000000000000000000000000000000000000666095e00000000000000000000000000000000000000000000000000000000000000000dcc3f422395fc31d9308eb3c4805623ddc445433eb04f7d4d7b07a9b4abb16886820d7c9a50f7bb450cff51271a9ff789322e9a72c65cf58da188c6b77093fdb1b00000000000000000000000000000000000000000000000000000000000000000000000000000000000042fff34f0b4b601ea1d21ac1184895b6d6b81662b95d14e59dfb768ef963838ca29f67dcaf0423b47312bd82d9f498976b28765bec3e79153ca76f644f04ef14dc001b000000000000000000000000000000000000000000000000000000000000c001a0ccd6f3e292c0acaea26b3fd6fee4bc1840fd38553b01637e01990ade4b6b26d4a05daf9fa73f7c0c0ae24097e01d04ed2d6548cd9a3668f8aa18abdb5eca623e08").into(),
            hex!("02f901920a820112830c5c06840af2724a830473c694a062ae8a9c5e11aaa026fc2670b0d65ccc8b285880b901245a47ddc3000000000000000000000000cb8fa9a76b8e203d8c3797bf438d8fb81ea3326a0000000000000000000000008ae125e8653821e851f12a49f7765db9a9ce73840000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000564edf7ae333278800000000000000000000000000000000000000000000000033f7ab48c542f25d000000000000000000000000000000000000000000000000564ca9d9ed92184200000000000000000000000000000000000000000000000033f656b5d849c5b30000000000000000000000004049d8f3f83365555e55e3594993fbeb30ccdc350000000000000000000000000000000000000000000000000000000066609a8ac080a071ef15fac388b7c5c9b56282610f0c7c5bde00ec3dcb07121fa04c64a0c53ccea0746f4a4cf21cf08f75ae7c078efcf148f910000986add1b7998d81874f5de009").into(),
        ];
        let payload_attrs = L2PayloadAttributes {
            fee_recipient: address!("4200000000000000000000000000000000000011"),
            gas_limit: Some(0x1c9c380),
            timestamp: 0x6660938b,
            prev_randao: b256!("22e77867678dc60aace7567ee344620f47a66be343eac90a82bf619ea37de357"),
            withdrawals: Default::default(),
            parent_beacon_block_root: Some(b256!(
                "50f4a35e2f059621cba649e719d23a2a9d030189fd19172a689c76d3adf39fec"
            )),
            transactions: raw_txs,
            no_tx_pool: false,
        };
        let produced_header = l2_block_executor.execute_payload(payload_attrs).unwrap().clone();

        assert_eq!(produced_header, expected_header);
        assert_eq!(l2_block_executor.parent_header.seal(), expected_header.hash_slow());
    }

    #[test]
    fn test_l2_block_executor_med_block_2() {
        // Static for the execution of block #121057303 on OP mainnet.
        // https://optimistic.etherscan.io/block/121057303/

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
        let raw_parent_header = hex!("f90245a071101c6ce251190d11965257bf7f3b079d5af139a80ec1d2541110ded5da9bd6a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a0df99471388344de2cff6b0ff98f9c66429c94f055d0aa4b96f5c5064c47e8ac0a0ebbb62603141a37336a38057ec8eca40e5aea904dafdff82a93c72d0ab9671cea05064f082249a9a7b00c8fc287a6e943b38ba6fe8e1fdc4bb0c10c89b9286a938b9010088000000c0120200100410c08048120b528040a00000000808840180040800201484b4c800040300208020c0001a08014040004021c0000028108018a980614100494020b00008004e020048800088004088094100094180406000c006564401001400005a00080006c0040348030a400a02810f08060104002410910001000011509000050a8200004000000820000280145a10a84000821000c080110020000404000000002e100090b0840000ac2214042040002024084081102800100010d1009226090008900820828280002400808d83a20000187001036005294c60085445800b8000410000a00200c1b19470000000049001052600300100020108808084073730168401c9c3808321106784666239e580a0d8ecef54b9a072a935b297c177b54dbbd5ee9e0fd811a2b69de4b1f28656ad16880000000000000000840392cf07a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a0fa918fbee01a47f475d70995e78b4505bd8714962012720cab27f7e66ec4ea5b");
        let parent_header = Header::decode(&mut &raw_parent_header[..]).unwrap();
        let raw_expected_header = hex!("f90245a0e2608bb1dd6e93302da709acfb82782ee2dcdcbaafdd07fa581958d4d0193560a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a0c8286187544a27fdd14372a0182b366be0c0f0f4c4a0a2ef31ee4538972266f5a08799d21d8d3e65106c57a16ea61b4d5ad8e440753b2788e1b8fdec17d6a88c72a06de5e10918168a54b43414e95a4c965baf0bf84c0c11c0711363f663a76c02b8b901000220004001000000000100000000000000000000000010000004000000000000000000c0008000000020001000000800000000000000200200002040000000000000080010000809000020080000000000040000000000000000000000008000000000000000000004000000020000200000000000000000020100100008002000000000000000000000000000000000000020000020000100000000000000000000001000000000000004000000040000000000000010000000000000100000000000020000040000000000000000000000000000000000000000000000000000000008000000000004000000000000000000000000081000000000000000008084073730178401c9c3808306757184666239e780a0d8ecef54b9a072a935b297c177b54dbbd5ee9e0fd811a2b69de4b1f28656ad16880000000000000000840390bc3da056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a0fa918fbee01a47f475d70995e78b4505bd8714962012720cab27f7e66ec4ea5b");
        let expected_header = Header::decode(&mut &raw_expected_header[..]).unwrap();

        // Initialize the block executor on block #121057302's post-state.
        let mut l2_block_executor = StatelessL2BlockExecutor::new(
            Arc::new(rollup_config),
            parent_header.seal_slow(),
            TestdataTrieDBFetcher::new("block_121057303_exec"),
        );

        let raw_txs = alloc::vec![
            hex!("7ef8f8a01a2c45522a69a90b583aa08a0968847a6fbbdc5480fe6f967b5fcb9384f46e9594deaddeaddeaddeaddeaddeaddeaddeaddead00019442000000000000000000000000000000000000158080830f424080b8a4440a5e2000000558000c5fc500000000000000010000000066623963000000000131b8d700000000000000000000000000000000000000000000000000000003ec02c0240000000000000000000000000000000000000000000000000000000000000001c10a3bb5847ad354f9a70b56f253baaea1c3841647851c4c62e10b22fe4e86940000000000000000000000006887246668a3b87f54deb3b94ba47a6f63f32985").into(),
            hex!("02f8b40a8316b3cf8405f5e100850bdfd63e00830249f09494b008aa00579c1307b0ef2c499ad98a8ce58e5880b844a9059cbb0000000000000000000000006713cbd38b831255b60b6c28cbdd15c769baad6d0000000000000000000000000000000000000000000000000000000024a12a1ec001a065ae43157da3a4f80cf3a63f572b408cde608af3f4cd98783d8277414d842b72a070caa5b8fcda2f1e9f40f8b310acbe57b95dbcd8f285775b7e53d783539beb94").into(),
            hex!("f9032d8301c3338406244dd88304c7fc941111111254eeb25477b68fb85ed929f73a96058280b902c412aa3caf000000000000000000000000b63aae6c353636d66df13b89ba4425cfe13d10ba000000000000000000000000420000000000000000000000000000000000000600000000000000000000000068f180fcce6836688e9084f035309e29bf0a2095000000000000000000000000b63aae6c353636d66df13b89ba4425cfe13d10ba0000000000000000000000003f343211f0487eb43af2e0e773ba012015e6651a000000000000000000000000000000000000000000000000074a17b261ebbf4000000000000000000000000000000000000000000000000000000000002b13e70000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000001800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001120000000000000000000000000000000000000000000000000000000000f400a0c9e75c48000000000000000020120000000000000000000000000000000000000000000000000000c600006302a000000000000000000000000000000000000000000000000000000000000f5b3fee63c1e581e1b9cc9cc17616ce81f0fa5b958d36f789fb2c0042000000000000000000000000000000000000061111111254eeb25477b68fb85ed929f73a96058202a000000000000000000000000000000000000000000000000000000000001b4ccdee63c1e58185c31ffa3706d1cce9d525a00f1c7d4a2911754c42000000000000000000000000000000000000061111111254eeb25477b68fb85ed929f73a960582000000000000000000000000000037a088fb0295e0b68236fa1742c8d1ee86d682e86928ce4b32f27c2010addbdb7020a01310030aba22db3e46766fb7bc3ba666535d25dfd9df5f13d55632ec8638d01b").into(),
            hex!("02f901d30a8303cd348316e36084608dcd0e8302cde8945800249621da520adfdca16da20d8a5fc0f814d880b901640ddedd8400000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000000000000000000000e00000000000000000000000000000000000000000000000000000000000000120000000000000000000000000000000000000000000000000000000000002d9f4000000000000000000000000000000000000000000000000005d423c655aa00000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000eb22708b72cc00b04346eee1767c0e147f8db2d00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000769127d620c000000000000000000000000000000000000000000000000000000000000000016692be0dfa2ce53a3d8c88ebcab639cf00c16197a717bc3ddeab46bbab181bbec001a0bdfb7260ed744771034511f4823380f16bb50427e1888f352c9c94d5d569e66da05cabb47cf62ed550d06af2f9555ff290f4b403fee7e32f67f19d3948db0dc1cb").into()
        ];
        let payload_attrs = L2PayloadAttributes {
            fee_recipient: address!("4200000000000000000000000000000000000011"),
            gas_limit: Some(30_000_000),
            timestamp: 1717713383,
            prev_randao: b256!("d8ecef54b9a072a935b297c177b54dbbd5ee9e0fd811a2b69de4b1f28656ad16"),
            withdrawals: Default::default(),
            parent_beacon_block_root: Some(b256!(
                "fa918fbee01a47f475d70995e78b4505bd8714962012720cab27f7e66ec4ea5b"
            )),
            transactions: raw_txs,
            no_tx_pool: false,
        };
        let produced_header = l2_block_executor.execute_payload(payload_attrs).unwrap().clone();

        assert_eq!(produced_header, expected_header);
        assert_eq!(l2_block_executor.parent_header.seal(), expected_header.hash_slow());
    }
}
