//! CallData Source

use crate::{
    traits::{AsyncIterator, ChainProvider},
    types::{BlockInfo, StageError, StageResult},
};
use alloc::{boxed::Box, collections::VecDeque};
use alloy_primitives::{Address, Bytes};
use async_trait::async_trait;

/// A data iterator that reads from calldata.
#[derive(Debug, Clone)]
pub struct CalldataSource<CP>
where
    CP: ChainProvider + Send,
{
    /// The chain provider to use for the calldata source.
    chain_provider: CP,
    /// The address of the batcher contract.
    batcher_address: Address,
    /// Block Ref
    block_ref: BlockInfo,
    /// The L1 Signer.
    signer: Address,
    /// Current calldata.
    calldata: VecDeque<Bytes>,
    /// Whether the calldata source is open.
    open: bool,
}

impl<CP: ChainProvider + Send> CalldataSource<CP> {
    /// Creates a new calldata source.
    pub fn new(
        chain_provider: CP,
        batcher_address: Address,
        block_ref: BlockInfo,
        signer: Address,
    ) -> Self {
        Self {
            chain_provider,
            batcher_address,
            block_ref,
            signer,
            calldata: VecDeque::new(),
            open: false,
        }
    }

    /// Loads the calldata into the source if it is not open.
    async fn load_calldata(&mut self) -> anyhow::Result<()> {
        if self.open {
            return Ok(());
        }

        let (_, txs) =
            self.chain_provider.block_info_and_transactions_by_hash(self.block_ref.hash).await?;

        self.calldata = txs
            .iter()
            .filter_map(|tx| {
                if tx.to() != Some(self.batcher_address) {
                    return None;
                }
                if tx.from() != Some(self.signer) {
                    return None;
                }
                Some(tx.data())
            })
            .collect::<VecDeque<_>>();

        self.open = true;

        Ok(())
    }
}

#[async_trait]
impl<CP: ChainProvider + Send> AsyncIterator for CalldataSource<CP> {
    type Item = Bytes;

    async fn next(&mut self) -> Option<StageResult<Self::Item>> {
        if self.load_calldata().await.is_err() {
            return Some(Err(StageError::BlockFetch(self.block_ref.hash)));
        }
        Some(self.calldata.pop_front().ok_or(StageError::Eof))
    }
}
