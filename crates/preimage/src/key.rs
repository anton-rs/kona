//! Contains the [PreimageKey] type, which is used to identify preimages that may be fetched from
//! the preimage oracle.

use crate::errors::InvalidPreimageKeyType;
use alloy_primitives::{B256, U256};
#[cfg(feature = "rkyv")]
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
#[cfg(feature = "serde")]
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

/// <https://specs.optimism.io/experimental/fault-proof/index.html#pre-image-key-types>
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
#[repr(u8)]
#[cfg_attr(feature = "rkyv", derive(Archive, RkyvSerialize, RkyvDeserialize))]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub enum PreimageKeyType {
    /// Local key types are local to a given instance of a fault-proof and context dependent.
    /// Commonly these local keys are mapped to bootstrap data for the fault proof program.
    Local = 1,
    /// Keccak256 key types are global and context independent. Preimages are mapped from the
    /// low-order 31 bytes of the preimage's `keccak256` digest to the preimage itself.
    #[default]
    Keccak256 = 2,
    /// GlobalGeneric key types are reserved for future use.
    GlobalGeneric = 3,
    /// Sha256 key types are global and context independent. Preimages are mapped from the
    /// low-order 31 bytes of the preimage's `sha256` digest to the preimage itself.
    Sha256 = 4,
    /// Blob key types are global and context independent. Blob keys are constructed as
    /// `keccak256(commitment ++ z)`, and then the high-order byte of the digest is set to the
    /// type byte.
    Blob = 5,
    /// Precompile key types are global and context independent. Precompile keys are constructed as
    /// `keccak256(precompile_addr ++ input)`, and then the high-order byte of the digest is set to
    /// the type byte.
    Precompile = 6,
}

impl TryFrom<u8> for PreimageKeyType {
    type Error = InvalidPreimageKeyType;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let key_type = match value {
            1 => PreimageKeyType::Local,
            2 => PreimageKeyType::Keccak256,
            3 => PreimageKeyType::GlobalGeneric,
            4 => PreimageKeyType::Sha256,
            5 => PreimageKeyType::Blob,
            6 => PreimageKeyType::Precompile,
            _ => return Err(InvalidPreimageKeyType),
        };
        Ok(key_type)
    }
}

/// A preimage key is a 32-byte value that identifies a preimage that may be fetched from the
/// oracle.
///
/// **Layout**:
/// |  Bits   | Description |
/// |---------|-------------|
/// | [0, 1)  | Type byte   |
/// | [1, 32) | Data        |
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "rkyv", derive(Archive, RkyvSerialize, RkyvDeserialize))]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct PreimageKey {
    data: [u8; 31],
    key_type: PreimageKeyType,
}

impl PreimageKey {
    /// Creates a new [PreimageKey] from a 32-byte value and a [PreimageKeyType]. The 32-byte value
    /// will be truncated to 31 bytes by taking the low-order 31 bytes.
    pub fn new(key: [u8; 32], key_type: PreimageKeyType) -> Self {
        let mut data = [0u8; 31];
        data.copy_from_slice(&key[1..]);
        Self { data, key_type }
    }

    /// Creates a new local [PreimageKey] from a 64-bit local identifier. The local identifier will
    /// be written into the low-order 8 bytes of the big-endian 31-byte data field.
    pub fn new_local(local_ident: u64) -> Self {
        let mut data = [0u8; 31];
        data[23..].copy_from_slice(&local_ident.to_be_bytes());
        Self { data, key_type: PreimageKeyType::Local }
    }

    /// Returns the [PreimageKeyType] for the [PreimageKey].
    pub fn key_type(&self) -> PreimageKeyType {
        self.key_type
    }

    /// Returns the value of the [PreimageKey] as a [U256].
    pub fn key_value(&self) -> U256 {
        U256::from_be_slice(self.data.as_slice())
    }
}

impl From<PreimageKey> for [u8; 32] {
    fn from(key: PreimageKey) -> Self {
        let mut rendered_key = [0u8; 32];
        rendered_key[0] = key.key_type as u8;
        rendered_key[1..].copy_from_slice(&key.data);
        rendered_key
    }
}

impl From<PreimageKey> for B256 {
    fn from(value: PreimageKey) -> Self {
        let raw: [u8; 32] = value.into();
        B256::from(raw)
    }
}

impl TryFrom<[u8; 32]> for PreimageKey {
    type Error = InvalidPreimageKeyType;

    fn try_from(value: [u8; 32]) -> Result<Self, Self::Error> {
        let key_type = PreimageKeyType::try_from(value[0])?;
        Ok(Self::new(value, key_type))
    }
}

impl core::fmt::Display for PreimageKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let raw: [u8; 32] = (*self).into();
        write!(f, "{}", B256::from(raw))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_preimage_keys() {
        let types = [
            PreimageKeyType::Local,
            PreimageKeyType::Keccak256,
            PreimageKeyType::GlobalGeneric,
            PreimageKeyType::Sha256,
            PreimageKeyType::Blob,
            PreimageKeyType::Precompile,
        ];

        for key_type in types {
            let key = PreimageKey::new([0xFFu8; 32], key_type);
            assert_eq!(key.key_type(), key_type);

            let mut rendered_key = [0xFFu8; 32];
            rendered_key[0] = key_type as u8;
            let actual: [u8; 32] = key.into();
            assert_eq!(actual, rendered_key);
        }
    }
}
