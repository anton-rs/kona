//! CallData Source

use crate::traits::{ChainProvider, DataIter};
use crate::types::BlockInfo;
use crate::types::StageResult;
use alloy_primitives::{Address, Bytes};

/// A data iterator that reads from calldata.
#[derive(Debug, Clone)]
pub struct CalldataSource {
    /// The chain provider to use for the calldata source.
    chain_provider: ChainProvider,
    /// The address of the batcher contract.
    batcher_address: Address,
    /// Block Ref
    block_ref: BlockInfo,
}

impl CalldataSource {
    /// Creates a new calldata source.
    pub fn new(
        chain_provider: ChainProvider,
        batcher_address: Address,
        block_ref: BlockInfo,
    ) -> Self {
        Self {
            chain_provider,
            batcher_address,
            block_ref,
        }
    }
}

impl<T: Into<Bytes>> DataIter<T> for CalldataSource {
    fn next(&mut self) -> StageResult<T> {
        unimplemented!()
    }
}
