//! Types for the pre-state claims used in the interop proof.

use alloc::vec::Vec;
use alloy_primitives::{keccak256, B256};
use alloy_rlp::{Buf, Decodable, Encodable, RlpDecodable, RlpEncodable};
use kona_interop::{SuperRoot, SUPER_ROOT_VERSION};

/// The current [TransitionState] encoding format version.
pub const TRANSITION_STATE_VERSION: u8 = 255;

/// The [PreState] of the interop proof program can be one of two types: a [SuperRoot] or a
/// [TransitionState]. The [SuperRoot] is the canonical state of the superchain, while the
/// [TransitionState] is a super-structure of the [SuperRoot] that represents the progress of a
/// pending superchain state transition from one [SuperRoot] to the next.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
pub enum PreState {
    /// The canonical state of the superchain.
    SuperRoot(SuperRoot),
    /// The progress of a pending superchain state transition.
    TransitionState(TransitionState),
}

impl Encodable for PreState {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self {
            Self::SuperRoot(super_root) => {
                super_root.encode(out);
            }
            Self::TransitionState(transition_state) => {
                transition_state.encode(out);
            }
        }
    }
}

impl Decodable for PreState {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        if buf.is_empty() {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        match buf[0] {
            TRANSITION_STATE_VERSION => {
                let transition_state = TransitionState::decode(buf)?;
                Ok(Self::TransitionState(transition_state))
            }
            SUPER_ROOT_VERSION => {
                let super_root =
                    SuperRoot::decode(buf).map_err(|_| alloy_rlp::Error::UnexpectedString)?;
                Ok(Self::SuperRoot(super_root))
            }
            _ => Err(alloy_rlp::Error::Custom("invalid version byte")),
        }
    }
}

/// The [TransitionState] is a super-structure of the [SuperRoot] that represents the progress of a
/// pending superchain state transition from one [SuperRoot] to the next.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
pub struct TransitionState {
    /// The canonical pre-state super root commitment.
    pub pre_state: SuperRoot,
    /// The progress that has been made in the pending superchain state transition.
    pub pending_progress: Vec<OptimisticBlock>,
    /// The step number of the pending superchain state transition.
    pub step: u64,
}

impl TransitionState {
    /// Create a new [TransitionState] with the given pre-state, pending progress, and step number.
    pub const fn new(
        pre_state: SuperRoot,
        pending_progress: Vec<OptimisticBlock>,
        step: u64,
    ) -> Self {
        Self { pre_state, pending_progress, step }
    }

    /// Hashes the encoded [TransitionState] using [keccak256].
    pub fn hash(&self) -> B256 {
        let mut rlp_buf = Vec::with_capacity(self.length());
        self.encode(&mut rlp_buf);
        keccak256(&rlp_buf)
    }
}

impl Encodable for TransitionState {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        out.put_u8(TRANSITION_STATE_VERSION);

        // The pre-state has special encoding, since it is not RLP. We encode the structure, and
        // then encode it as a RLP string.
        let mut pre_state_buf = Vec::with_capacity(self.pre_state.encoded_length());
        self.pre_state.encode(&mut pre_state_buf);
        pre_state_buf.encode(out);

        self.pending_progress.encode(out);
        self.step.encode(out);
    }

    fn length(&self) -> usize {
        self.pre_state.encoded_length() + self.pending_progress.length() + self.step.length()
    }
}

impl Decodable for TransitionState {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        if buf.is_empty() {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        let version = buf[0];
        if version != TRANSITION_STATE_VERSION {
            return Err(alloy_rlp::Error::Custom("invalid version byte"));
        }
        buf.advance(1);

        // The pre-state has special decoding, since it is not RLP. We decode the RLP string, and
        // then decode the structure.
        let pre_state_buf = Vec::<u8>::decode(buf)?;
        let pre_state = SuperRoot::decode(&mut pre_state_buf.as_slice())
            .map_err(|_| alloy_rlp::Error::UnexpectedString)?;

        // The rest of the fields are RLP encoded as normal.
        let pending_progress = Vec::<OptimisticBlock>::decode(buf)?;
        let step = u64::decode(buf)?;

        Ok(Self { pre_state, pending_progress, step })
    }
}

/// A wrapper around a pending output root hash with the block hash it commits to.
#[derive(Default, Debug, Clone, Eq, PartialEq, RlpEncodable, RlpDecodable)]
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
pub struct OptimisticBlock {
    /// The block hash of the output root.
    pub block_hash: B256,
    /// The output root hash.
    pub output_root: B256,
}

impl OptimisticBlock {
    /// Create a new [OptimisticBlock] with the given block hash and output root hash.
    pub const fn new(block_hash: B256, output_root: B256) -> Self {
        Self { block_hash, output_root }
    }
}

#[cfg(test)]
mod test {
    use super::{OptimisticBlock, SuperRoot, TransitionState};
    use alloy_primitives::B256;
    use alloy_rlp::{Decodable, Encodable};
    use arbitrary::Arbitrary;
    use kona_interop::OutputRootWithChain;
    use rand::Rng;

    #[test]
    fn test_static_transition_state_roundtrip() {
        let transition_state = TransitionState::new(
            SuperRoot::new(
                10,
                vec![
                    (OutputRootWithChain::new(1, B256::default())),
                    (OutputRootWithChain::new(2, B256::default())),
                ],
            ),
            vec![OptimisticBlock::default(), OptimisticBlock::default()],
            1,
        );

        let mut rlp_buf = Vec::with_capacity(transition_state.length());
        transition_state.encode(&mut rlp_buf);

        assert_eq!(transition_state, TransitionState::decode(&mut rlp_buf.as_slice()).unwrap());
    }

    #[test]
    fn test_arbitrary_pre_state_roundtrip() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());
        let pre_state =
            super::PreState::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();

        let mut rlp_buf = Vec::with_capacity(pre_state.length());
        pre_state.encode(&mut rlp_buf);
        assert_eq!(pre_state, super::PreState::decode(&mut rlp_buf.as_slice()).unwrap());
    }

    #[test]
    fn test_arbitrary_transition_state_roundtrip() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());
        let transition_state =
            TransitionState::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();

        let mut rlp_buf = Vec::with_capacity(transition_state.length());
        transition_state.encode(&mut rlp_buf);
        assert_eq!(transition_state, TransitionState::decode(&mut rlp_buf.as_slice()).unwrap());
    }
}
