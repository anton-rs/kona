//! Contains the logic for the EIP-7251 syscall.

use crate::{
    db::TrieDB,
    errors::{ExecutorError, ExecutorResult},
    TrieDBProvider,
};
use alloc::{boxed::Box, vec::Vec};
use alloy_primitives::{Address, Bytes, U256};
use kona_mpt::TrieHinter;
use maili_genesis::RollupConfig;
use op_alloy_rpc_types_engine::OpPayloadAttributes;
use revm::{
    db::State,
    primitives::{
        BlockEnv, CfgEnvWithHandlerCfg, Env, EnvWithHandlerCfg, OptimismFields, TransactTo, TxEnv,
    },
    DatabaseCommit, Evm,
};

/// Execute the EIP-7251 pre-block beacon root contract call.
pub(crate) fn pre_block_consolidation_requests_contract_call<F, H>(
    db: &mut State<&mut TrieDB<F, H>>,
    config: &RollupConfig,
    initialized_cfg: &CfgEnvWithHandlerCfg,
    initialized_block_env: &BlockEnv,
    payload: &OpPayloadAttributes,
) -> ExecutorResult<()>
where
    F: TrieDBProvider,
    H: TrieHinter,
{
    let mut evm_pre_block = Evm::builder()
        .with_db(db)
        .with_env_with_handler_cfg(EnvWithHandlerCfg::new_with_cfg_env(
            initialized_cfg.clone(),
            initialized_block_env.clone(),
            Default::default(),
        ))
        .build();

    // initialize a block from the env, because the pre block call needs the block itself
    apply_consolidation_requests_contract_call(
        config,
        payload.payload_attributes.timestamp,
        &mut evm_pre_block,
    )
}

/// Apply the EIP-7251 pre-block consolidation requests contract call to a given EVM instance.
fn apply_consolidation_requests_contract_call<F, H>(
    config: &RollupConfig,
    timestamp: u64,
    evm: &mut Evm<'_, (), &mut State<&mut TrieDB<F, H>>>,
) -> ExecutorResult<()>
where
    F: TrieDBProvider,
    H: TrieHinter,
{
    if !config.is_isthmus_active(timestamp) {
        return Ok(());
    }

    // Get the previous environment
    let previous_env = Box::new(evm.context.evm.env().clone());

    // modify env for pre block call
    fill_tx_env_with_consolidation_requests_contract_call(&mut evm.context.evm.env);

    let mut state = match evm.transact() {
        Ok(res) => res.state,
        Err(e) => {
            evm.context.evm.env = previous_env;
            return Err(ExecutorError::ExecutionError(e));
        }
    };

    state.remove(&alloy_eips::eip7002::SYSTEM_ADDRESS);
    state.remove(&evm.block().coinbase);

    evm.context.evm.db.commit(state);

    // re-set the previous env
    evm.context.evm.env = previous_env;

    Ok(())
}

/// Fill transaction environment with the EIP-7251 system contract message data.
///
/// This requirements for the beacon root contract call are defined by
/// [EIP-7251](https://eips.ethereum.org/EIPS/eip-7251).
fn fill_tx_env_with_consolidation_requests_contract_call(env: &mut Env) {
    fill_tx_env_with_system_contract_call(
        env,
        alloy_eips::eip7002::SYSTEM_ADDRESS,
        alloy_eips::eip7251::CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS,
        Bytes::new(),
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
            // The L1 fee is not charged for the EIP-7251 transaction, submit zero bytes for the
            // enveloped tx size.
            enveloped_tx: Some(Bytes::default()),
        },
    };

    // ensure the block gas limit is >= the tx
    env.block.gas_limit = U256::from(env.tx.gas_limit);

    // disable the base fee check for this call by setting the base fee to zero
    env.block.basefee = U256::ZERO;
}
