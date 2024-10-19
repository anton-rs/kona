//! Protocol constants for the executor.

use alloy_primitives::{address, Address};

/// The address of the fee recipient.
pub(crate) const FEE_RECIPIENT: Address = address!("4200000000000000000000000000000000000011");

/// The address of the L2 to L1 bridge predeploy.
pub(crate) const L2_TO_L1_BRIDGE: Address = address!("4200000000000000000000000000000000000016");

/// The current version of the output root format.
pub(crate) const OUTPUT_ROOT_VERSION: u8 = 0;

/// The version byte for the Holocene extra data.
pub(crate) const HOLOCENE_EXTRA_DATA_VERSION: u8 = 0x00;
