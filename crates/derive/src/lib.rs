#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use core::fmt::Debug;
use traits::{ChainProvider, TelemetryProvider};
use types::RollupConfig;

mod params;
pub use params::{
    ChannelID, CHANNEL_ID_LENGTH, CONFIG_UPDATE_EVENT_VERSION_0, CONFIG_UPDATE_TOPIC,
    DERIVATION_VERSION_0, FRAME_OVERHEAD, MAX_CHANNEL_BANK_SIZE, MAX_FRAME_LEN,
    MAX_RLP_BYTES_PER_CHANNEL, MAX_SPAN_BATCH_BYTES, SEQUENCER_FEE_VAULT_ADDRESS,
};

pub mod sources;
pub mod stages;
pub mod traits;
pub mod types;

/// The derivation pipeline is responsible for deriving L2 inputs from L1 data.
#[derive(Debug, Clone, Copy)]
pub struct DerivationPipeline;

impl DerivationPipeline {
    /// Creates a new instance of the [DerivationPipeline].
    pub fn new<P, T>(
        _rollup_config: Arc<RollupConfig>,
        _chain_provider: P,
        _telemetry: Arc<T>,
    ) -> Self
    where
        P: ChainProvider + Clone + Debug + Send,
        T: TelemetryProvider + Clone + Debug + Send + Sync,
    {
        // let l1_traversal = L1Traversal::new(chain_provider, rollup_config.clone(),
        // telemetry.clone()); let l1_retrieval = L1Retrieval::new(l1_traversal, dap_source,
        // telemetry.clone()); let frame_queue = FrameQueue::new(l1_retrieval,
        // telemetry.clone()); let channel_bank = ChannelBank::new(rollup_config.clone(),
        // frame_queue, telemetry.clone()); let channel_reader =
        // ChannelReader::new(channel_bank, telemetry.clone()); let batch_queue =
        // BatchQueue::new(rollup_config.clone(), channel_reader, telemetry.clone(), fetcher);
        // let attributes_queue = AttributesQueue::new(rollup_config.clone(), batch_queue,
        // telemetry.clone(), builder);

        unimplemented!()
    }
}
