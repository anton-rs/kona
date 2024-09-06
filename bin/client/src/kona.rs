#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]
#![cfg_attr(any(target_arch = "mips", target_arch = "riscv64"), no_main)]

extern crate alloc;

use alloc::sync::Arc;
use alloy_consensus::Header;
use kona_client::{
    l1::{DerivationDriver, OracleBlobProvider, OracleL1ChainProvider},
    l2::OracleL2ChainProvider,
    BootInfo, CachingOracle,
};
use kona_common_proc::client_entry;
use kona_executor::StatelessL2BlockExecutor;
use kona_primitives::L2AttributesWithParent;

pub(crate) mod fault;
use fault::{fpvm_handle_register, HINT_WRITER, ORACLE_READER};

/// The size of the LRU cache in the oracle.
const ORACLE_LRU_SIZE: usize = 1024;

#[client_entry(100_000_000)]
fn main() -> Result<()> {
    #[cfg(feature = "tracing-subscriber")]
    {
        use anyhow::anyhow;
        use tracing::Level;

        let subscriber = tracing_subscriber::fmt().with_max_level(Level::DEBUG).finish();
        tracing::subscriber::set_global_default(subscriber).map_err(|e| anyhow!(e))?;
    }

    kona_common::block_on(async move {
        ////////////////////////////////////////////////////////////////
        //                          PROLOGUE                          //
        ////////////////////////////////////////////////////////////////

        let oracle = Arc::new(CachingOracle::new(ORACLE_LRU_SIZE, ORACLE_READER, HINT_WRITER));
        let boot = Arc::new(BootInfo::load(oracle.as_ref()).await?);
        let l1_provider = OracleL1ChainProvider::new(boot.clone(), oracle.clone());
        let l2_provider = OracleL2ChainProvider::new(boot.clone(), oracle.clone());
        let beacon = OracleBlobProvider::new(oracle.clone());

        ////////////////////////////////////////////////////////////////
        //                   DERIVATION & EXECUTION                   //
        ////////////////////////////////////////////////////////////////

        let mut driver = DerivationDriver::new(
            boot.as_ref(),
            oracle.as_ref(),
            beacon,
            l1_provider,
            l2_provider.clone(),
        )
        .await?;
        let L2AttributesWithParent { attributes, .. } =
            driver.produce_disputed_payload().await?.expect("Failed to derive payload attributes");

        let mut executor = StatelessL2BlockExecutor::builder(&boot.rollup_config)
            .with_parent_header(driver.take_l2_safe_head_header())
            .with_fetcher(l2_provider.clone())
            .with_hinter(l2_provider)
            .with_handle_register(fpvm_handle_register)
            .build()?;
        let Header { number, .. } = *executor.execute_payload(attributes)?;
        let output_root = executor.compute_output_root()?;

        ////////////////////////////////////////////////////////////////
        //                          EPILOGUE                          //
        ////////////////////////////////////////////////////////////////

        assert_eq!(number, boot.l2_claim_block);
        assert_eq!(output_root, boot.l2_claim);

        tracing::info!(
            target: "client",
            "Successfully validated L2 block #{number} with output root {output_root}",
            number = number,
            output_root = output_root
        );

        kona_common::io::print(&alloc::format!(
            "Successfully validated L2 block #{} with output root {}\n",
            number,
            output_root
        ));

        Ok::<_, anyhow::Error>(())
    })
}
