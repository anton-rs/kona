//! CallData Source

use crate::traits::{AsyncIterator, ChainProvider};
use crate::types::BlockInfo;
use crate::types::StageError;
use crate::types::StageResult;
use alloc::boxed::Box;
use alloc::vec::Vec;
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
    calldata: Vec<Bytes>,
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
            calldata: Vec::new(),
            open: false,
        }
    }

    /// Loads the calldata into the source if it is not open.
    async fn load_calldata(&mut self) -> anyhow::Result<()> {
        if self.open {
            return Ok(());
        }

        let info = self
            .chain_provider
            .block_info_and_transactions_by_hash(self.block_ref.hash)
            .await?;

        self.calldata = info
            .1
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
            .collect::<Vec<_>>();

        self.open = true;

        Ok(())
    }
}

#[async_trait]
impl<CP: ChainProvider + Send> AsyncIterator for CalldataSource<CP> {
    type Item = StageResult<Bytes>;

    async fn next(&mut self) -> Option<Self::Item> {
        if self.load_calldata().await.is_err() {
            return Some(Err(StageError::BlockFetch(self.block_ref.hash)));
        }

        Some(self.calldata.pop().ok_or(StageError::Eof))
    }
}
