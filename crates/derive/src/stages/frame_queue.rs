//! This module contains the [FrameQueue] stage of the derivation pipeline.

use super::l1_retrieval::L1Retrieval;
use crate::traits::{ChainProvider, DataAvailabilityProvider};
use alloc::collections::VecDeque;
use alloy_primitives::Bytes;

pub struct FrameQueue<T, DAP, CP>
where
    DAP: DataAvailabilityProvider,
    CP: ChainProvider,
{
    /// The previous stage in the pipeline.
    pub prev: L1Retrieval<T, DAP, CP>,
    /// The current frame queue.
    queue: VecDeque<Bytes>,
}
