//! This module contains the [SignedRecoverable] trait.
//!
//! This trait exists to allow for alternative implementations of the `recover_public_key` method
//! for signed types that can supply the original message hash for public key recovery. By default,
//! it is implemented for [alloy_consensus::TxEnvelope] if the `k256` feature is enabled.

use alloy_primitives::Address;
use anyhow::Result;

#[cfg(feature = "k256")]
use alloy_consensus::TxEnvelope;
#[cfg(feature = "k256")]
use alloy_primitives::{Signature, B256};
#[cfg(feature = "k256")]
use anyhow::anyhow;

/// Represents a signed transaction that can be recovered.
pub trait SignedRecoverable {
    /// Recovers the public key from the signature and the message hash.
    fn recover_public_key(&self) -> Result<Address>;
}

#[cfg(feature = "k256")]
impl SignedRecoverable for TxEnvelope {
    fn recover_public_key(&self) -> Result<Address> {
        match self {
            TxEnvelope::Legacy(signed_tx) => {
                recover_public_key(*signed_tx.signature(), &signed_tx.signature_hash())
            }
            TxEnvelope::Eip2930(signed_tx) => {
                recover_public_key(*signed_tx.signature(), &signed_tx.signature_hash())
            }
            TxEnvelope::Eip1559(signed_tx) => {
                recover_public_key(*signed_tx.signature(), &signed_tx.signature_hash())
            }
            TxEnvelope::Eip4844(signed_tx) => {
                recover_public_key(*signed_tx.signature(), &signed_tx.signature_hash())
            }
            _ => unreachable!("Impossible case"),
        }
    }
}

/// Recovers the public key from a signature and a message hash.
#[cfg(feature = "k256")]
#[inline]
fn recover_public_key(sig: Signature, message_hash: &B256) -> Result<Address> {
    sig.recover_address_from_prehash(message_hash).map_err(|e| anyhow!(e))
}
