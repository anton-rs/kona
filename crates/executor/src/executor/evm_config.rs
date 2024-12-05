use reth_evm::{ConfigureEvmEnv, NextBlockEnvAttributes};
use reth_primitives::TransactionSigned;
use reth_primitives_traits::FillTxEnv;
use reth_optimism_chainspec::{OpChainSpec, DecodeError};
use reth_optimism_forks::OpHardfork;
use alloy_consensus::{Sealed, Header as AlloyHeader};
use alloy_primitives::{U256, Address, Bytes, TxKind};
use alloc::{sync::Arc, vec::Vec};
use revm_primitives::{
    CfgEnv, BlobExcessGasAndPrice, TxEnv, SpecId, HandlerCfg,
    BlockEnv, AnalysisKind, CfgEnvWithHandlerCfg
};
use revm::primitives::{Env, OptimismFields};

/// Trait for configuring the EVM for custom execution.
pub trait KonaEvmConfig: ConfigureEvmEnv {
    /// Create a new instance of the EVM config.
    fn new(chain_spec: Arc<OpChainSpec>) -> Self;

    /// Localize a Sealed Alloy Header into the local Header type.
    fn localize_alloy_header(header: &Sealed<AlloyHeader>) -> Self::Header;
}


#[derive(Clone, Debug)]
pub struct DefaultEvmConfig {
    chain_spec: Arc<OpChainSpec>,
}

impl KonaEvmConfig for DefaultEvmConfig {
    fn new(chain_spec: Arc<OpChainSpec>) -> Self {
        Self { chain_spec }
    }

    fn localize_alloy_header(header: &Sealed<AlloyHeader>) -> <DefaultEvmConfig as ConfigureEvmEnv>::Header {
        header.inner().clone()
    }
}

impl ConfigureEvmEnv for DefaultEvmConfig {
    type Header = AlloyHeader;
    type Error = DecodeError;

    /// Fill transaction environment from a [`TransactionSigned`] and the given sender address.
    fn fill_tx_env(&self, tx_env: &mut TxEnv, transaction: &TransactionSigned, sender: Address) {
        transaction.fill_tx_env(tx_env, sender);
    }

    /// Fill transaction environment with a system contract call.
    fn fill_tx_env_system_contract_call(
        &self,
        env: &mut Env,
        caller: Address,
        contract: Address,
        data: Bytes,
    ) {
        env.tx = TxEnv {
            caller,
            transact_to: TxKind::Call(contract),
            // Explicitly set nonce to None so revm does not do any nonce checks
            nonce: None,
            gas_limit: 30_000_000,
            value: U256::ZERO,
            data,
            // Setting the gas price to zero enforces that no value is transferred as part of the
            // call, and that the call will not count against the block's gas limit
            gas_price: U256::ZERO,
            // The chain ID check is not relevant here and is disabled if set to None
            chain_id: None,
            // Setting the gas priority fee to None ensures the effective gas price is derived from
            // the `gas_price` field, which we need to be zero
            gas_priority_fee: None,
            access_list: Vec::new(),
            // blob fields can be None for this tx
            blob_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            authorization_list: None,
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

    /// Fill [`CfgEnvWithHandlerCfg`] fields according to the chain spec and given header.
    ///
    /// This must set the corresponding spec id in the handler cfg, based on timestamp or total
    /// difficulty
    fn fill_cfg_env(
        &self,
        cfg_env: &mut CfgEnvWithHandlerCfg,
        header: &Self::Header,
        _total_difficulty: U256,
    ) {
        let spec_id = revm_spec_by_timestamp_after_bedrock(&self.chain_spec, header.timestamp);

        cfg_env.chain_id = self.chain_spec.chain().id();
        cfg_env.perf_analyse_created_bytecodes = AnalysisKind::Analyse;

        cfg_env.handler_cfg.spec_id = spec_id;
        cfg_env.handler_cfg.is_optimism = true;
    }

    /// Returns the configured [`CfgEnvWithHandlerCfg`] and [`BlockEnv`] for `parent + 1` block.
    ///
    /// This is intended for usage in block building after the merge and requires additional
    /// attributes that can't be derived from the parent block: attributes that are determined by
    /// the CL, such as the timestamp, suggested fee recipient, and randomness value.
    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> Result<(CfgEnvWithHandlerCfg, BlockEnv), Self::Error> {
        // configure evm env based on parent block
        let cfg = CfgEnv::default().with_chain_id(self.chain_spec.chain().id());

        // ensure we're not missing any timestamp based hardforks
        let spec_id = revm_spec_by_timestamp_after_bedrock(&self.chain_spec, attributes.timestamp);

        // if the parent block did not have excess blob gas (i.e. it was pre-cancun), but it is
        // cancun now, we need to set the excess blob gas to the default value(0)
        let blob_excess_gas_and_price = parent
            .next_block_excess_blob_gas()
            .or_else(|| (spec_id.is_enabled_in(SpecId::CANCUN)).then_some(0))
            .map(BlobExcessGasAndPrice::new);

        let block_env = BlockEnv {
            number: U256::from(parent.number + 1),
            coinbase: attributes.suggested_fee_recipient,
            timestamp: U256::from(attributes.timestamp),
            difficulty: U256::ZERO,
            prevrandao: Some(attributes.prev_randao),
            gas_limit: U256::from(parent.gas_limit),
            // calculate basefee based on parent block's gas usage
            basefee: self.chain_spec.next_block_base_fee(parent, attributes.timestamp)?,
            // calculate excess gas based on parent block's blob gas usage
            blob_excess_gas_and_price,
        };

        // ZTODO: Understand plugging in OSAKA here for features, where else?
        let cfg_with_handler_cfg;
        {
            cfg_with_handler_cfg = CfgEnvWithHandlerCfg {
                cfg_env: cfg,
                handler_cfg: HandlerCfg { spec_id, is_optimism: true },
            };
        }

        Ok((cfg_with_handler_cfg, block_env))
    }
}


fn revm_spec_by_timestamp_after_bedrock(
    chain_spec: &OpChainSpec,
    timestamp: u64,
) -> SpecId {
    if chain_spec.fork(OpHardfork::Holocene).active_at_timestamp(timestamp) {
        revm_primitives::HOLOCENE
    } else if chain_spec.fork(OpHardfork::Granite).active_at_timestamp(timestamp) {
        revm_primitives::GRANITE
    } else if chain_spec.fork(OpHardfork::Fjord).active_at_timestamp(timestamp) {
        revm_primitives::FJORD
    } else if chain_spec.fork(OpHardfork::Ecotone).active_at_timestamp(timestamp) {
        revm_primitives::ECOTONE
    } else if chain_spec.fork(OpHardfork::Canyon).active_at_timestamp(timestamp) {
        revm_primitives::CANYON
    } else if chain_spec.fork(OpHardfork::Regolith).active_at_timestamp(timestamp) {
        revm_primitives::REGOLITH
    } else {
        revm_primitives::BEDROCK
    }
}
