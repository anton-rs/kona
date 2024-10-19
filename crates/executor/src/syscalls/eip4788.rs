//! Contains the logic for executing the pre-block beacon root call.

use crate::{
    db::TrieDB,
    errors::{ExecutorError, ExecutorResult},
};
use alloc::{boxed::Box, vec::Vec};
use alloy_eips::eip4788::BEACON_ROOTS_ADDRESS;
use alloy_primitives::{Address, Bytes, B256, U256};
use kona_mpt::{TrieHinter, TrieProvider};
use op_alloy_genesis::RollupConfig;
use op_alloy_rpc_types_engine::OpPayloadAttributes;
use revm::{
    db::State,
    primitives::{
        BlockEnv, CfgEnvWithHandlerCfg, Env, EnvWithHandlerCfg, OptimismFields, TransactTo, TxEnv,
    },
    DatabaseCommit, Evm,
};

/// Execute the EIP-4788 pre-block beacon root contract call.
pub(crate) fn pre_block_beacon_root_contract_call<F, H>(
    db: &mut State<&mut TrieDB<F, H>>,
    config: &RollupConfig,
    block_number: u64,
    initialized_cfg: &CfgEnvWithHandlerCfg,
    initialized_block_env: &BlockEnv,
    payload: &OpPayloadAttributes,
) -> ExecutorResult<()>
where
    F: TrieProvider,
    H: TrieHinter,
{
    // apply pre-block EIP-4788 contract call
    let mut evm_pre_block = Evm::builder()
        .with_db(db)
        .with_env_with_handler_cfg(EnvWithHandlerCfg::new_with_cfg_env(
            initialized_cfg.clone(),
            initialized_block_env.clone(),
            Default::default(),
        ))
        .build();

    // initialize a block from the env, because the pre block call needs the block itself
    apply_beacon_root_contract_call(
        config,
        payload.payload_attributes.timestamp,
        block_number,
        payload.payload_attributes.parent_beacon_block_root,
        &mut evm_pre_block,
    )
}

/// Apply the EIP-4788 pre-block beacon root contract call to a given EVM instance.
fn apply_beacon_root_contract_call<F, H>(
    config: &RollupConfig,
    timestamp: u64,
    block_number: u64,
    parent_beacon_block_root: Option<B256>,
    evm: &mut Evm<'_, (), &mut State<&mut TrieDB<F, H>>>,
) -> ExecutorResult<()>
where
    F: TrieProvider,
    H: TrieHinter,
{
    if !config.is_ecotone_active(timestamp) {
        return Ok(());
    }

    let parent_beacon_block_root =
        parent_beacon_block_root.ok_or(ExecutorError::MissingParentBeaconBlockRoot)?;

    // if the block number is zero (genesis block) then the parent beacon block root must
    // be 0x0 and no system transaction may occur as per EIP-4788
    if block_number == 0 {
        if parent_beacon_block_root != B256::ZERO {
            return Err(ExecutorError::MissingParentBeaconBlockRoot);
        }
        return Ok(());
    }

    // Get the previous environment
    let previous_env = Box::new(evm.context.evm.env().clone());

    // modify env for pre block call
    fill_tx_env_with_beacon_root_contract_call(&mut evm.context.evm.env, parent_beacon_block_root);

    let mut state = match evm.transact() {
        Ok(res) => res.state,
        Err(e) => {
            evm.context.evm.env = previous_env;
            return Err(ExecutorError::ExecutionError(e));
        }
    };

    state.remove(&alloy_eips::eip4788::SYSTEM_ADDRESS);
    state.remove(&evm.block().coinbase);

    evm.context.evm.db.commit(state);

    // re-set the previous env
    evm.context.evm.env = previous_env;

    Ok(())
}

/// Fill transaction environment with the EIP-4788 system contract message data.
///
/// This requirements for the beacon root contract call defined by
/// [EIP-4788](https://eips.ethereum.org/EIPS/eip-4788) are:
///
/// At the start of processing any execution block where `block.timestamp >= FORK_TIMESTAMP` (i.e.
/// before processing any transactions), call [`BEACON_ROOTS_ADDRESS`] as
/// [`SYSTEM_ADDRESS`](alloy_eips::eip4788::SYSTEM_ADDRESS) with the 32-byte input of
/// `header.parent_beacon_block_root`. This will trigger the `set()` routine of the beacon roots
/// contract.
fn fill_tx_env_with_beacon_root_contract_call(env: &mut Env, parent_beacon_block_root: B256) {
    fill_tx_env_with_system_contract_call(
        env,
        alloy_eips::eip4788::SYSTEM_ADDRESS,
        BEACON_ROOTS_ADDRESS,
        parent_beacon_block_root.0.into(),
    );
}

/// Fill transaction environment with the system caller and the system contract address and message
/// data.
///
/// This is a system operation and therefore:
///  * the call must execute to completion
///  * the call does not count against the block’s gas limit
///  * the call does not follow the EIP-1559 burn semantics - no value should be transferred as part
///    of the call
///  * if no code exists at the provided address, the call will fail silently
fn fill_tx_env_with_system_contract_call(
    env: &mut Env,
    caller: Address,
    contract: Address,
    data: Bytes,
) {
    env.tx = TxEnv {
        caller,
        transact_to: TransactTo::Call(contract),
        // Explicitly set nonce to None so revm does not do any nonce checks
        nonce: None,
        gas_limit: 30_000_000,
        value: U256::ZERO,
        data,
        // Setting the gas price to zero enforces that no value is transferred as part of the call,
        // and that the call will not count against the block's gas limit
        gas_price: U256::ZERO,
        // The chain ID check is not relevant here and is disabled if set to None
        chain_id: None,
        // Setting the gas priority fee to None ensures the effective gas price is derived from the
        // `gas_price` field, which we need to be zero
        gas_priority_fee: None,
        access_list: Vec::new(),
        authorization_list: None,
        // blob fields can be None for this tx
        blob_hashes: Vec::new(),
        max_fee_per_blob_gas: None,
        optimism: OptimismFields {
            source_hash: None,
            mint: None,
            is_system_transaction: Some(false),
            // The L1 fee is not charged for the EIP-4788 transaction, submit zero bytes for the
            // enveloped tx size.
            enveloped_tx: Some(Bytes::default()),
        },
    };

    // ensure the block gas limit is >= the tx
    env.block.gas_limit = U256::from(env.tx.gas_limit);

    // disable the base fee check for this call by setting the base fee to zero
    env.block.basefee = U256::ZERO;
}
