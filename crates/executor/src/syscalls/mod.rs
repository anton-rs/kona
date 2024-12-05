//! Optimism EVM System calls.

mod eip4788;
pub(crate) use eip4788::apply_beacon_root_contract_call;

mod canyon;
pub(crate) use canyon::ensure_create2_deployer_canyon;
