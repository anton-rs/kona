//! Optimism EVM System calls.

mod eip2935;
pub(crate) use eip2935::pre_block_block_hash_contract_call;

mod eip7002;
pub(crate) use eip7002::pre_block_withdrawals_request_contract_call;

mod eip7251;
pub(crate) use eip7251::pre_block_consolidation_requests_contract_call;

mod eip4788;
pub(crate) use eip4788::pre_block_beacon_root_contract_call;

mod canyon;
pub(crate) use canyon::ensure_create2_deployer_canyon;
