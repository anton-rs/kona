//! An implementation of the [DataAvailabilityProvider] trait for tests.

use crate::{
    errors::{PipelineError, PipelineResult},
    traits::{AsyncIterator, DataAvailabilityProvider},
};
use alloc::{boxed::Box, vec, vec::Vec};
use alloy_primitives::{Address, Bytes};
use async_trait::async_trait;
use core::fmt::Debug;
use op_alloy_protocol::BlockInfo;

/// Mock data iterator
#[derive(Debug, Default, PartialEq)]
pub struct TestIter {
    /// Holds open data calls with args for assertions.
    pub(crate) open_data_calls: Vec<(BlockInfo, Address)>,
    /// A queue of results to return as the next iterated data.
    pub(crate) results: Vec<PipelineResult<Bytes>>,
}

#[async_trait]
impl AsyncIterator for TestIter {
    type Item = Bytes;

    async fn next(&mut self) -> PipelineResult<Self::Item> {
        self.results.pop().unwrap_or(Err(PipelineError::Eof.temp()))
    }
}

/// Mock data availability provider
#[derive(Debug, Default)]
pub struct TestDAP {
    /// The batch inbox address.
    pub batch_inbox_address: Address,
    /// Specifies the stage results the test iter returns as data.
    pub(crate) results: Vec<PipelineResult<Bytes>>,
}

#[async_trait]
impl DataAvailabilityProvider for TestDAP {
    type Item = Bytes;
    type DataIter = TestIter;

    async fn open_data(&self, block_ref: &BlockInfo) -> PipelineResult<Self::DataIter> {
        // Construct a new vec of results to return.
        let results = self
            .results
            .iter()
            .map(|i| i.as_ref().map_or_else(|_| Err(PipelineError::Eof.temp()), |r| Ok(r.clone())))
            .collect::<Vec<PipelineResult<Bytes>>>();
        Ok(TestIter { open_data_calls: vec![(*block_ref, self.batch_inbox_address)], results })
    }
}
