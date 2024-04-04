//! Testing utilities for the attributes queue stage.

use crate::{
    stages::attributes_queue::AttributesBuilder,
    types::{BlockID, L2BlockInfo, PayloadAttributes},
};
use alloc::vec::Vec;

/// A mock implementation of the [`AttributesBuilder`] for testing.
#[derive(Debug, Default)]
pub struct MockAttributesBuilder {
    /// The attributes to return.
    pub attributes: Vec<anyhow::Result<PayloadAttributes>>,
}

impl AttributesBuilder for MockAttributesBuilder {
    /// Prepares the [PayloadAttributes] for the next payload.
    fn prepare_payload_attributes(
        &mut self,
        _l2_parent: L2BlockInfo,
        _epoch: BlockID,
    ) -> anyhow::Result<PayloadAttributes> {
        self.attributes.pop().ok_or(anyhow::anyhow!("missing payload attribute"))?
    }
}
