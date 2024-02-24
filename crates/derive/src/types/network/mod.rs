//! `alloy-network` crate ported to `no_std`.

use crate::types::eips::eip2718::Eip2718Envelope;
use alloc::vec::Vec;
use alloy_primitives::B256;

mod sealed;
pub use sealed::{Sealable, Sealed};

mod transaction;
pub use transaction::{Eip1559Transaction, Signed, Transaction, TxKind};

mod receipt;
pub use receipt::Receipt;

/// A list of transactions, either hydrated or hashes.
// #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
// #[serde(untagged)]
pub enum TransactionList<T> {
    /// Hashes only.
    Hashes(Vec<B256>),
    /// Hydrated tx objects.
    Hydrated(Vec<T>),
    /// Special case for uncle response
    Uncled,
}
