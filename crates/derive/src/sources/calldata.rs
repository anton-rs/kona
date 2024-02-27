//! CallData Source

use crate::traits::ChainProvider;
use crate::types::BlockInfo;
use crate::types::StageResult;
use alloy_primitives::{Address, Bytes};

/// A data iterator that reads from calldata.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CalldataSource<CP>
where
    CP: ChainProvider,
{
    /// The chain provider to use for the calldata source.
    chain_provider: CP,
    /// The address of the batcher contract.
    batcher_address: Address,
    /// Block Ref
    block_ref: BlockInfo,
}

impl<CP: ChainProvider> CalldataSource<CP> {
    /// Creates a new calldata source.
    pub fn new(chain_provider: CP, batcher_address: Address, block_ref: BlockInfo) -> Self {
        Self {
            chain_provider,
            batcher_address,
            block_ref,
        }
    }
}

impl<CP: ChainProvider> Iterator for CalldataSource<CP> {
    type Item = StageResult<Bytes>;

    fn next(&mut self) -> Option<Self::Item> {
        unimplemented!()
    }
}
