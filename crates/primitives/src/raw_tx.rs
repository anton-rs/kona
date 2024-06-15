//! Contains the [RawTransaction] type.

use alloy_primitives::Bytes;
use alloy_rlp::{Decodable, Encodable};

/// A raw transaction
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct RawTransaction(pub Bytes);

impl RawTransaction {
    /// Returns if the transaction is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns if the transaction is a deposit
    pub fn is_deposit(&self) -> bool {
        !self.0.is_empty() && self.0[0] == 0x7E
    }
}

impl<T: Into<Bytes>> From<T> for RawTransaction {
    fn from(bytes: T) -> Self {
        Self(bytes.into())
    }
}

impl Encodable for RawTransaction {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.0.encode(out)
    }
}

impl Decodable for RawTransaction {
    /// Decodes RLP encoded bytes into [RawTransaction] bytes
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let tx_bytes = Bytes::decode(buf)?;
        Ok(Self(tx_bytes))
    }
}

impl AsRef<[u8]> for RawTransaction {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
