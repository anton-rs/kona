use crate::types::{
    eips::eip2718::{Decodable2718, Eip2718Error, Encodable2718},
    network::Signed,
    transaction::{TxDeposit, TxEip1559, TxEip2930, TxEip4844, TxLegacy},
};
use alloy_primitives::{Address, Bytes};
use alloy_rlp::{length_of_length, Decodable, Encodable};

/// Ethereum `TransactionType` flags as specified in EIPs [2718], [1559], and
/// [2930].
///
/// [2718]: https://eips.ethereum.org/EIPS/eip-2718
/// [1559]: https://eips.ethereum.org/EIPS/eip-1559
/// [2930]: https://eips.ethereum.org/EIPS/eip-2930
/// [4844]: https://eips.ethereum.org/EIPS/eip-4844
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Default)]
pub enum TxType {
    /// Wrapped legacy transaction type.
    #[default]
    Legacy = 0,
    /// EIP-2930 transaction type.
    Eip2930 = 1,
    /// EIP-1559 transaction type.
    Eip1559 = 2,
    /// EIP-4844 transaction type.
    Eip4844 = 3,
    /// Optimism Deposit transaction type.
    Deposit = 126,
}

impl TryFrom<u8> for TxType {
    type Error = Eip2718Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            // SAFETY: repr(u8) with explicit discriminant
            ..=3 | 126 => Ok(unsafe { core::mem::transmute(value) }),
            _ => Err(Eip2718Error::UnexpectedType(value)),
        }
    }
}

/// The Ethereum [EIP-2718] Transaction Envelope.
///
/// # Note:
///
/// This enum distinguishes between tagged and untagged legacy transactions, as
/// the in-protocol merkle tree may commit to EITHER 0-prefixed or raw.
/// Therefore we must ensure that encoding returns the precise byte-array that
/// was decoded, preserving the presence or absence of the `TransactionType`
/// flag.
///
/// [EIP-2718]: https://eips.ethereum.org/EIPS/eip-2718
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxEnvelope {
    /// An untagged [`TxLegacy`].
    Legacy(Signed<TxLegacy>),
    /// A [`TxLegacy`] tagged with type 0.
    TaggedLegacy(Signed<TxLegacy>),
    /// A [`TxEip2930`].
    Eip2930(Signed<TxEip2930>),
    /// A [`TxEip1559`].
    Eip1559(Signed<TxEip1559>),
    /// A [`TxEip4844`].
    Eip4844(Signed<TxEip4844>),
    /// A [`TxDeposit`].
    Deposit(Signed<TxDeposit>),
}

impl From<Signed<TxEip2930>> for TxEnvelope {
    fn from(v: Signed<TxEip2930>) -> Self {
        Self::Eip2930(v)
    }
}

impl From<Signed<TxEip1559>> for TxEnvelope {
    fn from(v: Signed<TxEip1559>) -> Self {
        Self::Eip1559(v)
    }
}

impl TxEnvelope {
    /// Returns the inner transaction `to` field.
    pub fn to(&self) -> Option<Address> {
        match self {
            Self::Legacy(t) | Self::TaggedLegacy(t) => t.to.to(),
            Self::Eip2930(t) => t.to.to(),
            Self::Eip1559(t) => t.to.to(),
            Self::Eip4844(t) => t.to.to(),
            Self::Deposit(t) => t.to.to(),
        }
    }

    /// Returns the inner transaction `from` field.
    pub fn from(&self) -> Option<Address> {
        // TODO(refcell): fix this to work for non-k256
        #[cfg(feature = "k256")]
        match self {
            Self::Legacy(t) | Self::TaggedLegacy(t) => t.recover_signer().ok(),
            Self::Eip2930(t) => t.recover_signer().ok(),
            Self::Eip1559(t) => t.recover_signer().ok(),
            Self::Eip4844(t) => t.recover_signer().ok(),
            Self::Deposit(t) => Some(t.from),
        }
        #[cfg(not(feature = "k256"))]
        return None;
    }

    /// Returns the inner transaction data.
    pub fn data(&self) -> Bytes {
        match self {
            Self::Legacy(t) | Self::TaggedLegacy(t) => t.input.clone(),
            Self::Eip2930(t) => t.input.clone(),
            Self::Eip1559(t) => t.input.clone(),
            Self::Eip4844(t) => t.input.clone(),
            Self::Deposit(t) => t.input.clone(),
        }
    }

    /// Return the [`TxType`] of the inner txn.
    pub const fn tx_type(&self) -> TxType {
        match self {
            Self::Legacy(_) | Self::TaggedLegacy(_) => TxType::Legacy,
            Self::Eip2930(_) => TxType::Eip2930,
            Self::Eip1559(_) => TxType::Eip1559,
            Self::Eip4844(_) => TxType::Eip4844,
            Self::Deposit(_) => TxType::Deposit,
        }
    }

    /// Return the length of the inner txn.
    pub fn inner_length(&self) -> usize {
        match self {
            Self::Legacy(t) | Self::TaggedLegacy(t) => t.length(),
            Self::Eip2930(t) => t.length(),
            Self::Eip1559(t) => t.length(),
            Self::Eip4844(t) => t.length(),
            Self::Deposit(t) => t.length(),
        }
    }

    /// Return the RLP payload length of the network-serialized wrapper
    fn rlp_payload_length(&self) -> usize {
        if let Self::Legacy(t) = self {
            return t.length();
        }
        // length of inner tx body
        let inner_length = self.inner_length();
        // with tx type byte
        inner_length + 1
    }
}

impl Encodable for TxEnvelope {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.network_encode(out)
    }

    fn length(&self) -> usize {
        let mut payload_length = self.rlp_payload_length();
        if !self.is_legacy() {
            payload_length += length_of_length(payload_length);
        }
        payload_length
    }
}

impl Decodable for TxEnvelope {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        match Self::network_decode(buf) {
            Ok(t) => Ok(t),
            Err(Eip2718Error::RlpError(e)) => Err(e),
            Err(_) => Err(alloy_rlp::Error::Custom("Unexpected type")),
        }
    }
}

impl Decodable2718 for TxEnvelope {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Result<Self, Eip2718Error> {
        match ty.try_into()? {
            TxType::Legacy => Ok(Self::TaggedLegacy(
                Decodable::decode(buf).map_err(Eip2718Error::RlpError)?,
            )),
            TxType::Eip2930 => Ok(Self::Eip2930(
                Decodable::decode(buf).map_err(Eip2718Error::RlpError)?,
            )),
            TxType::Eip1559 => Ok(Self::Eip1559(
                Decodable::decode(buf).map_err(Eip2718Error::RlpError)?,
            )),
            TxType::Eip4844 => Ok(Self::Eip4844(
                Decodable::decode(buf).map_err(Eip2718Error::RlpError)?,
            )),
            TxType::Deposit => Ok(Self::Deposit(
                Decodable::decode(buf).map_err(Eip2718Error::RlpError)?,
            )),
        }
    }

    fn fallback_decode(buf: &mut &[u8]) -> Result<Self, Eip2718Error> {
        Ok(TxEnvelope::Legacy(
            Decodable::decode(buf).map_err(Eip2718Error::RlpError)?,
        ))
    }
}

impl Encodable2718 for TxEnvelope {
    fn type_flag(&self) -> Option<u8> {
        match self {
            Self::Legacy(_) => None,
            Self::TaggedLegacy(_) => Some(TxType::Legacy as u8),
            Self::Eip2930(_) => Some(TxType::Eip2930 as u8),
            Self::Eip1559(_) => Some(TxType::Eip1559 as u8),
            Self::Eip4844(_) => Some(TxType::Eip4844 as u8),
            Self::Deposit(_) => Some(TxType::Deposit as u8),
        }
    }

    fn encode_2718_len(&self) -> usize {
        self.inner_length() + !self.is_legacy() as usize
    }

    fn encode_2718(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            TxEnvelope::Legacy(tx) => tx.encode(out),
            TxEnvelope::TaggedLegacy(tx) => {
                out.put_u8(TxType::Legacy as u8);
                tx.encode(out);
            }
            TxEnvelope::Eip2930(tx) => {
                out.put_u8(TxType::Eip2930 as u8);
                tx.encode(out);
            }
            TxEnvelope::Eip1559(tx) => {
                out.put_u8(TxType::Eip1559 as u8);
                tx.encode(out);
            }
            TxEnvelope::Eip4844(tx) => {
                out.put_u8(TxType::Eip4844 as u8);
                tx.encode(out);
            }
            TxEnvelope::Deposit(tx) => {
                out.put_u8(TxType::Deposit as u8);
                tx.encode(out);
            }
        }
    }
}
