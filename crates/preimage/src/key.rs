//! Contains the [PreimageKey] type, which is used to identify preimages that may be fetched from the preimage oracle.

/// <https://github.com/ethereum-optimism/optimism/blob/develop/specs/fault-proof.md#pre-image-key-types>
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum PreimageKeyType {
    /// Local key types are local to a given instance of a fault-proof and context dependent. Commonly these local keys
    /// are mapped to bootstrap data for the fault proof program.
    Local = 1,
    /// Keccak256 key types are global and context independent. Preimages are mapped from the low-order 31 bytes of
    /// the preimage's `keccak256` digest to the preimage itself.
    #[default]
    Keccak256 = 2,
    /// GlobalGeneric key types are reserved for future use.
    GlobalGeneric = 3,
    /// Sha256 key types are global and context independent. Preimages are mapped from the low-order 31 bytes of
    /// the preimage's `sha256` digest to the preimage itself.
    Sha256 = 4,
    /// Blob key types are global and context independent. Blob keys are constructed as `keccak256(commitment ++ z)`,
    /// and then the high-order byte of the digest is set to the type byte.
    Blob = 5,
}

/// A preimage key is a 32-byte value that identifies a preimage that may be fetched from the oracle.
///
/// **Layout**:
/// |  Bits   | Description |
/// |---------|-------------|
/// | [0, 1)  | Type byte   |
/// | [1, 32) | Data        |
#[derive(Debug, Default, Clone, Copy)]
pub struct PreimageKey {
    data: [u8; 31],
    key_type: PreimageKeyType,
}

impl PreimageKey {
    /// Creates a new [PreimageKey] from a 32-byte value and a [PreimageKeyType]. The 32-byte value will be truncated
    /// to 31 bytes by taking the low-order 31 bytes.
    pub fn new(key: [u8; 32], key_type: PreimageKeyType) -> Self {
        let mut data = [0u8; 31];
        data.copy_from_slice(&key[1..]);
        Self { data, key_type }
    }

    /// Returns the [PreimageKeyType] for the [PreimageKey].
    pub fn key_type(&self) -> PreimageKeyType {
        self.key_type
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_local_key() {
        let types = [PreimageKeyType::Local, PreimageKeyType::Keccak256];

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
