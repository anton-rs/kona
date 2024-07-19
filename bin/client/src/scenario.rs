use crate::{
    l1::{DerivationDriver, OracleBlobProvider, OracleL1ChainProvider},
    l2::{FPVMPrecompileOverride, OracleL2ChainProvider, TrieDBHintWriter},
    BootInfo, CachingOracle,
};
use alloc::{sync::Arc, vec::Vec};
use alloy_consensus::{Header, Sealed};
use alloy_primitives::B256;
use anyhow::{Ok, Result};
use kona_executor::StatelessL2BlockExecutor;
use kona_preimage::PreimageKey;
use kona_primitives::{L2AttributesWithParent, L2PayloadAttributes};
use revm::primitives::HashMap;

type ExecutorType = StatelessL2BlockExecutor<
    OracleL2ChainProvider,
    TrieDBHintWriter,
    FPVMPrecompileOverride<OracleL2ChainProvider, TrieDBHintWriter>,
>;

/// Scenario of the client program.
#[derive(Debug)]
pub struct Scenario {
    oracle: Arc<CachingOracle>,
    /// Boot information.
    pub boot: Arc<BootInfo>,
    l1_provider: OracleL1ChainProvider,
    l2_provider: OracleL2ChainProvider,
    beacon: OracleBlobProvider,
    executor: Option<ExecutorType>,
}

impl Scenario {
    /// Prologue of the client program.
    pub async fn new(prebuilt_preimage: Option<HashMap<PreimageKey, Vec<u8>>>) -> Result<Self> {
        let oracle = Arc::new(CachingOracle::new(prebuilt_preimage));
        let boot = Arc::new(BootInfo::load(oracle.as_ref()).await.unwrap());
        let l1_provider = OracleL1ChainProvider::new(boot.clone(), oracle.clone());
        let l2_provider = OracleL2ChainProvider::new(boot.clone(), oracle.clone());
        let beacon = OracleBlobProvider::new(oracle.clone());

        Ok(Self { oracle, boot, l1_provider, l2_provider, beacon, executor: None })
    }

    ///
    pub async fn derive(&mut self) -> Result<(L2PayloadAttributes, Sealed<Header>)> {
        let mut driver = DerivationDriver::new(
            self.boot.as_ref(),
            self.oracle.as_ref(),
            self.beacon.clone(),
            self.l1_provider.clone(),
            self.l2_provider.clone(),
        )
        .await
        .unwrap();
        let L2AttributesWithParent { attributes, .. } =
            driver.produce_disputed_payload().await.unwrap();

        Ok((attributes, driver.take_l2_safe_head_header()))
    }

    /// Derivation and execution of the client program.
    pub async fn execute_block(
        &mut self,
        attributes: L2PayloadAttributes,
        l2_safe_head_header: Sealed<Header>,
    ) -> Result<u64> {
        let precompile_overrides =
            FPVMPrecompileOverride::<OracleL2ChainProvider, TrieDBHintWriter>::default();
        self.executor = Some(
            StatelessL2BlockExecutor::builder(self.boot.rollup_config.clone())
                .with_parent_header(l2_safe_head_header)
                .with_fetcher(self.l2_provider.clone())
                .with_hinter(TrieDBHintWriter)
                .with_precompile_overrides(precompile_overrides)
                .build()
                .unwrap(),
        );
        let Header { number, .. } =
            *self.executor.as_mut().unwrap().execute_payload(attributes).unwrap();
        Ok(number)
    }

    /// Compute the output root.
    /// TODO(ethan): it should be receipt height as a input.
    pub async fn compute_output_root(&mut self) -> Result<B256> {
        self.executor.as_mut().unwrap().compute_output_root()
    }
}
