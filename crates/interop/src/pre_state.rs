//! Contains standard definitions for the prestate formats used in the interop
//! proof.

use crate::{SUPER_ROOT_VERSION, TRANSITION_STATE_VERSION};
use alloc::vec::Vec;
use alloy_primitives::{keccak256, B256, U256};
use alloy_rlp::{Buf, Decodable, Encodable, RlpDecodable, RlpEncodable};

/// A wrapper around an output root hash with the chain ID it belongs to.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
pub struct OutputRootWithChain {
    /// The chain ID of the output root.
    pub chain_id: u64,
    /// The output root hash.
    pub output_root: B256,
}

impl OutputRootWithChain {
    /// Create a new [OutputRootWithChain] with the given chain ID and output root hash.
    pub fn new(chain_id: u64, output_root: B256) -> Self {
        Self { chain_id, output_root }
    }
}

/// A wrapper around an output root hash with the block hash it commits to.
#[derive(Default, Debug, Clone, Eq, PartialEq, RlpEncodable, RlpDecodable)]
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
pub struct OutputRootWithBlockHash {
    /// The block hash of the output root.
    pub block_hash: B256,
    /// The output root hash.
    pub output_root: B256,
}

impl OutputRootWithBlockHash {
    /// Create a new [OutputRootWithBlockHash] with the given block hash and output root hash.
    pub fn new(block_hash: B256, output_root: B256) -> Self {
        Self { block_hash, output_root }
    }
}

/// The [SuperRoot] is the snapshot of the superchain at a given timestamp.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(any(feature = "arbitrary", test), derive(arbitrary::Arbitrary))]
pub struct SuperRoot {
    /// The timestamp of the superchain snapshot, in seconds.
    pub timestamp: u64,
    /// The chain IDs and output root hashes of all chains within the dependency set.
    pub output_roots: Vec<OutputRootWithChain>,
}

impl SuperRoot {
    /// Create a new [SuperRoot] with the given timestamp and output roots.
    pub fn new(timestamp: u64, mut output_roots: Vec<OutputRootWithChain>) -> Self {
        // If the output roots are not sorted by chain ID, sort them in ascending order.
        if !output_roots.is_sorted_by_key(|r| r.chain_id) {
            output_roots.sort_by_key(|r| r.chain_id);
        }

        Self { timestamp, output_roots }
    }

    /// Hashes the encoded [SuperRoot] using [keccak256].
    pub fn hash(&self) -> B256 {
        let mut rlp_buf = Vec::with_capacity(self.length());
        self.encode(&mut rlp_buf);
        keccak256(&rlp_buf)
    }
}

impl Encodable for SuperRoot {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        out.put_u8(SUPER_ROOT_VERSION);

        out.put_slice(&self.timestamp.to_be_bytes());
        for output_root in &self.output_roots {
            out.put_slice(U256::from(output_root.chain_id).to_be_bytes::<32>().as_slice());
            out.put_slice(output_root.output_root.as_slice());
        }
    }

    fn length(&self) -> usize {
        8 + 64 * self.output_roots.len()
    }
}

impl Decodable for SuperRoot {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        if buf.is_empty() {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        let version = buf[0];
        if version != SUPER_ROOT_VERSION {
            return Err(alloy_rlp::Error::Custom("invalid version byte"));
        }
        buf.advance(1);

        if buf.len() < 8 {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }
        let timestamp = u64::from_be_bytes(buf[0..8].try_into().unwrap());
        buf.advance(8);

        let mut output_roots = Vec::new();
        while !buf.is_empty() {
            if buf.len() < 64 {
                return Err(alloy_rlp::Error::UnexpectedLength);
            }

            let chain_id = U256::from_be_bytes::<32>(buf[0..32].try_into().unwrap());
            buf.advance(32);
            let output_root = B256::from_slice(&buf[0..32]);
            buf.advance(32);
            output_roots.push(OutputRootWithChain::new(chain_id.to(), output_root));
        }

        Ok(Self { timestamp, output_roots })
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
    pub pending_progress: Vec<OutputRootWithBlockHash>,
    /// The step number of the pending superchain state transition.
    pub step: u64,
}

impl TransitionState {
    /// Create a new [TransitionState] with the given pre-state, pending progress, and step number.
    pub fn new(
        pre_state: SuperRoot,
        pending_progress: Vec<OutputRootWithBlockHash>,
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
        let mut pre_state_buf = Vec::with_capacity(self.pre_state.length());
        self.pre_state.encode(&mut pre_state_buf);
        pre_state_buf.encode(out);

        self.pending_progress.encode(out);
        self.step.encode(out);
    }

    fn length(&self) -> usize {
        self.pre_state.length() + self.pending_progress.length() + self.step.length()
    }
}

impl Decodable for TransitionState {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        if buf.is_empty() {
            return Err(alloy_rlp::Error::Custom("empty buffer"));
        }

        let version = buf[0];
        if version != TRANSITION_STATE_VERSION {
            return Err(alloy_rlp::Error::Custom("invalid version byte"));
        }
        buf.advance(1);

        // The pre-state has special decoding, since it is not RLP. We decode the RLP string, and
        // then decode the structure.
        let pre_state_buf = Vec::<u8>::decode(buf)?;
        let pre_state = SuperRoot::decode(&mut pre_state_buf.as_slice())?;

        // The rest of the fields are RLP encoded as normal.
        let pending_progress = Vec::<OutputRootWithBlockHash>::decode(buf)?;
        let step = u64::decode(buf)?;

        Ok(Self { pre_state, pending_progress, step })
    }
}

#[cfg(test)]
mod test {
    use crate::pre_state::OutputRootWithBlockHash;

    use super::{OutputRootWithChain, SuperRoot, TransitionState};
    use alloy_primitives::{b256, hex, B256};
    use alloy_rlp::{Decodable, Encodable};
    use arbitrary::Arbitrary;
    use rand::Rng;

    #[test]
    fn test_super_root_sorts_outputs() {
        let super_root = SuperRoot::new(
            10,
            vec![
                (OutputRootWithChain::new(2, B256::default())),
                (OutputRootWithChain::new(1, B256::default())),
            ],
        );

        assert!(super_root.output_roots.is_sorted_by_key(|r| r.chain_id));
    }

    #[test]
    fn test_static_super_root_roundtrip() {
        let super_root = SuperRoot::new(
            10,
            vec![
                (OutputRootWithChain::new(1, B256::default())),
                (OutputRootWithChain::new(2, B256::default())),
            ],
        );

        let mut rlp_buf = Vec::with_capacity(super_root.length());
        super_root.encode(&mut rlp_buf);

        assert_eq!(super_root, SuperRoot::decode(&mut rlp_buf.as_slice()).unwrap());
    }

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
            vec![OutputRootWithBlockHash::default(), OutputRootWithBlockHash::default()],
            1,
        );

        let mut rlp_buf = Vec::with_capacity(transition_state.length());
        transition_state.encode(&mut rlp_buf);

        assert_eq!(transition_state, TransitionState::decode(&mut rlp_buf.as_slice()).unwrap());
    }

    #[test]
    fn test_arbitrary_super_root_roundtrip() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());
        let super_root = SuperRoot::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();

        let mut rlp_buf = Vec::with_capacity(super_root.length());
        super_root.encode(&mut rlp_buf);
        assert_eq!(super_root, SuperRoot::decode(&mut rlp_buf.as_slice()).unwrap());
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

    #[test]
    fn testing() {
        let super_root = SuperRoot::new(
            1736453154,
            vec![OutputRootWithChain::new(
                11155420,
                b256!("eaeb38bfaac27182c837ffc70ac92cff1ff368868a4483493b10b4f73a0af402"),
            )],
        );

        let mut buf = Vec::new();
        super_root.encode(&mut buf);
        println!("{}", hex::encode(buf));

        let transition_state = TransitionState::new(
            super_root,
            vec![OutputRootWithBlockHash::new(
                b256!("e22926201cbc7d2039b973a22a475f1dc53cafa00b4eed3947d3fc9824f49194"),
                b256!("5e20ea68c11554da87d8c2377689b423b578d02a8172374599ab31821784e390"),
            )],
            1,
        );
        println!("{}", hex::encode(transition_state.hash().as_slice()));

        panic!();
    }
}
