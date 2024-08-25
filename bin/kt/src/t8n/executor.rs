//! `kona-executor` test executor.

use super::trie_fetcher::ExecutionFixtureTrieFetcher;
use alloy_consensus::{Header, EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};
use alloy_primitives::{address, Sealable};
use alloy_rlp::Encodable;
use anyhow::{ensure, Result};
use kona_derive::types::{L2PayloadAttributes, OP_MAINNET_CONFIG};
use kona_executor::{
    NoPrecompileOverride, StatelessL2BlockExecutor, StatelessL2BlockExecutorBuilder,
};
use op_test_vectors::execution::ExecutionFixture;

pub(crate) struct OptimismExecutor<'a> {
    /// The [ExecutionFixture] instance.
    fixture: &'a ExecutionFixture,
    /// The inner [StatelessL2BlockExecutor] instance.
    inner: StatelessL2BlockExecutor<
        'a,
        ExecutionFixtureTrieFetcher<'a>,
        ExecutionFixtureTrieFetcher<'a>,
        NoPrecompileOverride,
    >,
}

impl<'a> OptimismExecutor<'a> {
    /// Create a new [OptimismExecutor] instance.
    pub(crate) fn new(fixture: &'a ExecutionFixture) -> Result<Self> {
        // Construct the trie fetcher.
        let fetcher = ExecutionFixtureTrieFetcher::new(fixture)?;

        // Construct the partial starting header.
        let parent_header = Header {
            parent_hash: fixture.env.previous_hash,
            ommers_hash: EMPTY_OMMER_ROOT_HASH,
            beneficiary: address!("4200000000000000000000000000000000000011"),
            state_root: fetcher.root,
            transactions_root: EMPTY_ROOT_HASH,
            receipts_root: EMPTY_ROOT_HASH,
            withdrawals_root: Some(EMPTY_ROOT_HASH),
            number: fixture.env.current_number.to::<u64>() - 1,
            ..Default::default()
        }
        .seal_slow();

        let inner = StatelessL2BlockExecutorBuilder::with_config(&OP_MAINNET_CONFIG)
            .with_parent_header(parent_header)
            .with_fetcher(fetcher.clone())
            .with_hinter(fetcher)
            .with_precompile_overrides(NoPrecompileOverride)
            .build()?;

        Ok(Self { fixture, inner })
    }

    /// Execute the block in the fixture and check the results.
    pub(crate) fn execute_checked(&mut self) -> Result<()> {
        // RLP encode the transactions.
        let encoded_txs = self
            .fixture
            .transactions
            .iter()
            .map(|tx| {
                let mut buf = Vec::with_capacity(tx.length());
                tx.encode(&mut buf);
                buf.into()
            })
            .collect::<Vec<_>>();

        // Construct the payload attributes.
        let attrs = L2PayloadAttributes {
            timestamp: self.fixture.env.current_timestamp.to::<u64>(),
            prev_randao: self.fixture.env.current_difficulty.into(),
            fee_recipient: address!("4200000000000000000000000000000000000011"),
            transactions: encoded_txs,
            no_tx_pool: false,
            gas_limit: Some(self.fixture.env.current_gas_limit.to()),
            parent_beacon_block_root: Some(Default::default()), /* TODO: We need this in the
                                                                 * fixture. */
            ..Default::default()
        };

        // Execute the payload.
        let header = self.inner.execute_payload(attrs)?;

        // Perform final assertions on execution integrity.
        ensure!(header.state_root == self.fixture.result.state_root, "Invalid state root");
        ensure!(
            header.transactions_root == self.fixture.result.tx_root,
            "Invalid transactions root"
        );
        ensure!(header.receipts_root == self.fixture.result.receipt_root, "Invalid receipts root");
        ensure!(header.logs_bloom == self.fixture.result.logs_bloom, "Invalid logs bloom");

        Ok(())
    }
}
