//! This module contains the prologue phase of the client program, pulling in the boot information
//! through the `PreimageOracle` ABI as local keys.

use crate::errors::OracleProviderError;
use alloy_primitives::{B256, U256};
use kona_preimage::{PreimageKey, PreimageOracleClient};
use op_alloy_genesis::RollupConfig;
use op_alloy_registry::ROLLUP_CONFIGS;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// The local key ident for the L1 head hash.
pub const L1_HEAD_KEY: U256 = U256::from_be_slice(&[1]);

/// The local key ident for the L2 pre-state claim.
pub const AGREED_L2_PRE_STATE_KEY: U256 = U256::from_be_slice(&[2]);

/// The local key ident for the L2 post-state claim.
pub const CLAIMED_L2_POST_STATE_KEY: U256 = U256::from_be_slice(&[3]);

/// The local key ident for the L2 claim timestamp.
pub const L2_CLAIM_TIMESTAMP_KEY: U256 = U256::from_be_slice(&[4]);

/// The local key ident for the L2 chain ID.
pub const L2_CHAIN_ID_KEY: U256 = U256::from_be_slice(&[5]);

/// The local key ident for the L2 rollup config.
pub const L2_ROLLUP_CONFIG_KEY: U256 = U256::from_be_slice(&[6]);

/// The boot information for the interop client program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootInfo {
    /// The L1 head hash containing the safe L2 chain data that may reproduce the L2 head hash.
    pub l1_head: B256,
    /// The agreed upon superchain pre-state commitment.
    pub agreed_pre_state: B256,
    /// The claimed (disputed) superchain post-state commitment.
    pub claimed_post_state: B256,
    /// The L2 claim timestamp.
    pub claimed_l2_timestamp: u64,
    /// The L2 chain ID.
    pub chain_id: u64,
    /// The rollup config for the L2 chain.
    pub rollup_config: RollupConfig,
}

impl BootInfo {
    /// Load the boot information from the preimage oracle.
    ///
    /// ## Takes
    /// - `oracle`: The preimage oracle reader.
    ///
    /// ## Returns
    /// - `Ok(BootInfo)`: The boot information.
    /// - `Err(_)`: Failed to load the boot information.
    pub async fn load<O>(oracle: &O) -> Result<Self, OracleProviderError>
    where
        O: PreimageOracleClient + Send,
    {
        let mut l1_head: B256 = B256::ZERO;
        oracle
            .get_exact(PreimageKey::new_local(L1_HEAD_KEY.to()), l1_head.as_mut())
            .await
            .map_err(OracleProviderError::Preimage)?;

        let mut l2_pre: B256 = B256::ZERO;
        oracle
            .get_exact(PreimageKey::new_local(AGREED_L2_PRE_STATE_KEY.to()), l2_pre.as_mut())
            .await
            .map_err(OracleProviderError::Preimage)?;

        let mut l2_post: B256 = B256::ZERO;
        oracle
            .get_exact(PreimageKey::new_local(CLAIMED_L2_POST_STATE_KEY.to()), l2_post.as_mut())
            .await
            .map_err(OracleProviderError::Preimage)?;

        let l2_claim_block = u64::from_be_bytes(
            oracle
                .get(PreimageKey::new_local(L2_CLAIM_TIMESTAMP_KEY.to()))
                .await
                .map_err(OracleProviderError::Preimage)?
                .as_slice()
                .try_into()
                .map_err(OracleProviderError::SliceConversion)?,
        );
        let chain_id = u64::from_be_bytes(
            oracle
                .get(PreimageKey::new_local(L2_CHAIN_ID_KEY.to()))
                .await
                .map_err(OracleProviderError::Preimage)?
                .as_slice()
                .try_into()
                .map_err(OracleProviderError::SliceConversion)?,
        );

        // Attempt to load the rollup config from the chain ID. If there is no config for the chain,
        // fall back to loading the config from the preimage oracle.
        let rollup_config = if let Some(config) = ROLLUP_CONFIGS.get(&chain_id) {
            config.clone()
        } else {
            warn!(
                target: "boot-loader",
                "No rollup config found for chain ID {}, falling back to preimage oracle. This is insecure in production without additional validation!",
                chain_id
            );
            let ser_cfg = oracle
                .get(PreimageKey::new_local(L2_ROLLUP_CONFIG_KEY.to()))
                .await
                .map_err(OracleProviderError::Preimage)?;
            serde_json::from_slice(&ser_cfg).map_err(OracleProviderError::Serde)?
        };

        Ok(Self {
            l1_head,
            agreed_pre_state: l2_pre,
            claimed_post_state: l2_post,
            claimed_l2_timestamp: l2_claim_block,
            chain_id,
            rollup_config,
        })
    }
}

