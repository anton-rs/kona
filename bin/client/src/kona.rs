#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]
#![cfg_attr(any(target_arch = "mips", target_arch = "riscv64", target_os = "zkvm"), no_main)]

use kona_client::scenario::Scenario;
use kona_common_proc::client_entry;

extern crate alloc;

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
        let mut scenario = Scenario::new(None).await?;

        ////////////////////////////////////////////////////////////////
        //                   DERIVATION & EXECUTION                   //
        ////////////////////////////////////////////////////////////////
        let (attributes, l2_safe_head_header) = scenario.derive().await?;
        let number = scenario.execute_block(attributes, l2_safe_head_header).await?;
        let output_root = scenario.compute_output_root().await?;

        ////////////////////////////////////////////////////////////////
        //                          EPILOGUE                          //
        ////////////////////////////////////////////////////////////////
        assert_eq!(number, scenario.boot.l2_claim_block);
        assert_eq!(output_root, scenario.boot.l2_claim);

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
