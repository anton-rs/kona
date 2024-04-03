//! Test utilities for data availability.

use crate::{
    traits::{DataAvailabilityProvider, DataIter},
    types::{BlockInfo, StageError, StageResult},
};
use alloc::{boxed::Box, vec, vec::Vec};
use alloy_primitives::{Address, Bytes};
use anyhow::Result;
use async_trait::async_trait;
use core::fmt::Debug;

/// Mock data iterator
#[derive(Debug, Default, PartialEq)]
pub struct TestIter {
    /// Holds open data calls with args for assertions.
    pub(crate) open_data_calls: Vec<(BlockInfo, Address)>,
    /// A queue of results to return as the next iterated data.
    pub(crate) results: Vec<StageResult<Bytes>>,
}

impl DataIter<Bytes> for TestIter {
    fn next(&mut self) -> StageResult<Bytes> {
        self.results.pop().unwrap_or_else(|| Err(StageError::Eof))
    }
}

/// Mock data availability provider
#[derive(Debug, Default)]
pub struct TestDAP {
    /// Specifies the stage results the test iter returns as data.
    pub(crate) results: Vec<StageResult<Bytes>>,
}

#[async_trait]
impl DataAvailabilityProvider for TestDAP {
    type DataIter = TestIter;

    async fn open_data(
        &self,
        block_ref: &BlockInfo,
        batcher_address: Address,
    ) -> Result<Self::DataIter> {
        // Construct a new vec of results to return.
        let results = self
            .results
            .iter()
            .map(|i| match i {
                Ok(r) => Ok(r.clone()),
                Err(_) => Err(StageError::Eof),
            })
            .collect::<Vec<StageResult<Bytes>>>();
        Ok(TestIter { open_data_calls: vec![(*block_ref, batcher_address)], results })
    }
}
