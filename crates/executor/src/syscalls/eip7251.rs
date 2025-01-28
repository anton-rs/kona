//! Contains the logic for the EIP-7251 syscall.

use crate::{
    db::TrieDB,
    errors::{ExecutorError, ExecutorResult},
    syscalls::fill_tx_env_for_contract_call,
    TrieDBProvider,
};
use alloc::boxed::Box;
use alloy_primitives::Bytes;
use kona_mpt::TrieHinter;
use maili_genesis::RollupConfig;
use op_alloy_rpc_types_engine::OpPayloadAttributes;
use revm::{
    db::State,
    primitives::{BlockEnv, CfgEnvWithHandlerCfg, EnvWithHandlerCfg},
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
    fill_tx_env_for_contract_call(
        &mut evm.context.evm.env,
        alloy_eips::eip7002::SYSTEM_ADDRESS,
        alloy_eips::eip7251::CONSOLIDATION_REQUEST_PREDEPLOY_ADDRESS,
        Bytes::new(),
    );

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
