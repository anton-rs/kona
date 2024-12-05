//! Contains the logic for executing the pre-block beacon root call.

use crate::{
    db::TrieDB,
    errors::{ExecutorError, ExecutorResult},
    TrieDBProvider,
};
use alloc::{boxed::Box, vec::Vec};
use alloy_eips::eip4788::BEACON_ROOTS_ADDRESS;
use alloy_primitives::{Address, Bytes, B256, U256};
use kona_mpt::TrieHinter;
use op_alloy_genesis::RollupConfig;
use op_alloy_rpc_types_engine::OpPayloadAttributes;
use revm::{
    db::State,
    DatabaseCommit, Evm,
};
use reth_evm::ConfigureEvm;

/// Apply the EIP-4788 pre-block beacon root contract call to a given EVM instance.
pub fn apply_beacon_root_contract_call<F, H, C: ConfigureEvm>(
    config: &RollupConfig,
    evm_config: &C,
    timestamp: u64,
    block_number: u64,
    parent_beacon_block_root: Option<B256>,
    evm: &mut Evm<'_, (), &mut State<&mut TrieDB<F, H>>>,
) -> ExecutorResult<()>
where
    F: TrieDBProvider,
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
    evm_config.fill_tx_env_system_contract_call(
        // ZTODO: Current reth uses different version, need to sync them somehow
        &mut *evm.context.evm.env,
        alloy_eips::eip4788::SYSTEM_ADDRESS,
        BEACON_ROOTS_ADDRESS,
        parent_beacon_block_root.0.into(),
    );

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
