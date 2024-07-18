use alloy_primitives::{address, b256, uint};
use kona_primitives::{
    BlockID, ChainGenesis, RollupConfig, SystemConfig, OP_BASE_FEE_PARAMS,
    OP_CANYON_BASE_FEE_PARAMS,
};

// TODO(Ethan): replace the code with one that reads the rollup config from the config file.
pub(crate) const OP_DEVNET_CONFIG: RollupConfig = RollupConfig {
    genesis: ChainGenesis {
        l1: BlockID {
            hash: b256!("d8039aa6e09066e3a5cb5440dd65820e3af25f4cb5e98ed7e624c9043a407f08"),
            number: 0_u64,
        },
        l2: BlockID {
            hash: b256!("9ae58aeb7e33427050e536e7ea20b27a6123256e12ec3c8ba0e6931fbd3d5e3e"),
            number: 0_u64,
        },
        l2_time: 1721283024,
        system_config: Some(SystemConfig {
            batcher_addr: address!("3c44cdddb6a900fa2b585dd299e03d12fa4293bc"),
            overhead: uint!(0x834_U256),
            scalar: uint!(0xf4240_U256),
            gas_limit: 30_000_000_u64,
            base_fee_scalar: None,
            blob_base_fee_scalar: None,
        }),
        extra_data: None,
    },
    block_time: 2_u64,
    max_sequencer_drift: 300_u64,
    seq_window_size: 3600_u64,
    channel_timeout: 120_u64,
    l1_chain_id: 900_u64,
    l2_chain_id: 901_u64,
    base_fee_params: OP_BASE_FEE_PARAMS,
    canyon_base_fee_params: Some(OP_CANYON_BASE_FEE_PARAMS),
    regolith_time: Some(0_u64),
    canyon_time: Some(0_u64),
    delta_time: Some(0_u64),
    ecotone_time: Some(1_721_283_088_u64),
    fjord_time: None,
    interop_time: None,
    batch_inbox_address: address!("ff00000000000000000000000000000000000901"),
    deposit_contract_address: address!("6509f2a854ba7441039fce3b959d5badd2ffcfcd"),
    l1_system_config_address: address!("4af802b3010e07845b2b8c2250126e9ac0bdb6b9"),
    protocol_versions_address: address!("0000000000000000000000000000000000000000"),
    da_challenge_address: Some(address!("0000000000000000000000000000000000000000")),
    blobs_enabled_l1_timestamp: None,
};
