use alloc::boxed::Box;
use alloy_primitives::Bytes;
use core::fmt::Display;
use kona_primitives::BlockInfo;

/// A plasma error.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlasmaError {
    /// A reorg is required.
    ReorgRequired,
    /// Not enough data.
    NotEnoughData,
    /// The commitment was challenge, but the challenge period expired.
    ChallengeExpired,
    /// Missing data past the challenge period.
    MissingPastWindow,
    /// A challenge is pending for the given commitment
    ChallengePending,
}

impl Display for PlasmaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ReorgRequired => write!(f, "reorg required"),
            Self::NotEnoughData => write!(f, "not enough data"),
            Self::ChallengeExpired => write!(f, "challenge expired"),
            Self::MissingPastWindow => write!(f, "missing past window"),
            Self::ChallengePending => write!(f, "challenge pending"),
        }
    }
}

/// A callback method for the finalized head signal.
pub type FinalizedHeadSignal = Box<dyn Fn(BlockInfo) + Send>;

/// Max input size ensures the canonical chain cannot include input batches too large to
/// challenge in the Data Availability Challenge contract. Value in number of bytes.
/// This value can only be changed in a hard fork.
pub const MAX_INPUT_SIZE: usize = 130672;

/// TxDataVersion1 is the version number for batcher transactions containing
/// plasma commitments. It should not collide with DerivationVersion which is still
/// used downstream when parsing the frames.
pub const TX_DATA_VERSION_1: u8 = 1;

/// The default commitment type for the DA storage.
pub const KECCAK_256_COMMITMENT_TYPE: u8 = 0;

/// The default commitment type.
pub type Keccak256Commitment = Bytes;

/// DecodeKeccak256 validates and casts the commitment into a Keccak256Commitment.
pub fn decode_keccak256(commitment: &[u8]) -> Result<Keccak256Commitment, PlasmaError> {
    if commitment.is_empty() {
        return Err(PlasmaError::NotEnoughData);
    }
    if commitment[0] != KECCAK_256_COMMITMENT_TYPE {
        return Err(PlasmaError::NotEnoughData);
    }
    let c = &commitment[1..];
    if c.len() != 32 {
        return Err(PlasmaError::NotEnoughData);
    }
    Ok(Bytes::copy_from_slice(c))
}
