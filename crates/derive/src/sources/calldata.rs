//! CallData Source

use crate::{
    traits::{AsyncIterator, ChainProvider, SignedRecoverable},
    types::{BlockInfo, StageError, StageResult},
};
use alloc::{boxed::Box, collections::VecDeque};
use alloy_consensus::{Transaction, TxEnvelope};
use alloy_primitives::{Address, Bytes, TxKind};
use async_trait::async_trait;

/// A data iterator that reads from calldata.
#[derive(Debug, Clone)]
pub struct CalldataSource<CP>
where
    CP: ChainProvider + Send,
{
    /// The chain provider to use for the calldata source.
    chain_provider: CP,
    /// The batch inbox address.
    batcher_inbox_address: Address,
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
        batcher_inbox_address: Address,
        block_ref: BlockInfo,
        signer: Address,
    ) -> Self {
        Self {
            chain_provider,
            batcher_inbox_address,
            block_ref,
            signer,
            calldata: VecDeque::new(),
            open: false,
        }
    }

    /// Loads the calldata into the source if it is not open.
    async fn load_calldata(&mut self) -> anyhow::Result<()> {
        tracing::debug!("Loading calldata for block {}", self.block_ref.hash);
        if self.open {
            return Ok(());
        }

        let (_, txs) =
            self.chain_provider.block_info_and_transactions_by_hash(self.block_ref.hash).await?;

        self.calldata = txs
            .iter()
            .filter_map(|tx| {
                let (tx_kind, data) = match tx {
                    TxEnvelope::Legacy(tx) => (tx.tx().to(), tx.tx().input()),
                    TxEnvelope::Eip2930(tx) => (tx.tx().to(), tx.tx().input()),
                    TxEnvelope::Eip1559(tx) => (tx.tx().to(), tx.tx().input()),
                    _ => return None,
                };
                let TxKind::Call(to) = tx_kind else { return None };
                tracing::debug!("tx with calldata to: {}", to);

                if to != self.batcher_inbox_address {
                    return None;
                }
                tracing::debug!("tx sent to batcher inbox");
                if tx.recover_public_key().ok()? != self.signer {
                    return None;
                }
                tracing::debug!("tx signed by correct signer");
                Some(data.to_vec().into())
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
