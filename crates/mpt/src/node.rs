//! This module contains the [TrieNode] type, which allows for decoding the RLP

use alloc::{collections::VecDeque, vec::Vec};
use alloy_primitives::Bytes;
use alloy_rlp::{Buf, Decodable, EMPTY_LIST_CODE, EMPTY_STRING_CODE};
use anyhow::{anyhow, Result};

/// The length of the branch list when RLP encoded
const BRANCH_LIST_LENGTH: usize = 17;

/// The length of a leaf or extension node's RLP encoded list
const LEAF_OR_EXTENSION_LIST_LENGTH: usize = 2;

/// Prefix for even-nibbled extension node paths.
const PREFIX_EXTENSION_EVEN: u8 = 0;

/// Prefix for odd-nibbled extension node paths.
const PREFIX_EXTENSION_ODD: u8 = 1;

/// Prefix for even-nibbled leaf node paths.
const PREFIX_LEAF_EVEN: u8 = 2;

/// Prefix for odd-nibbled leaf node paths.
const PREFIX_LEAF_ODD: u8 = 3;

/// A [TrieNode] is a node within a standard Merkle Patricia Trie.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TrieNode {
    /// A branch node refers to up to 16 child nodes with the encoding `rlp([ v0, ..., v15, value
    /// ])`
    Branch {
        /// The 16 child nodes and value of the branch.
        stack: VecDeque<NodeElement>,
    },
    /// A leaf node is a 2-item node with the encoding `rlp([encoded_path, value])`
    Leaf {
        /// The key of the leaf node
        key: Bytes,
        /// The value of the leaf node
        value: Bytes,
    },
    /// An extension node is a 2-item pointer node with the encoding `rlp([encoded_path, key])`
    Extension {
        /// The path prefix of the extension
        prefix: Bytes,
        /// The pointer to the child node
        node: Bytes,
    },
}

impl TrieNode {
    /// Attempts to convert a `path` and `value` into a [TrieNode], if they correspond to a
    /// [TrieNode::Leaf] or [TrieNode::Extension].
    pub fn try_from_path_and_value(path: Bytes, value: Bytes) -> Result<Self> {
        match path[0] >> 4 {
            PREFIX_EXTENSION_EVEN | PREFIX_EXTENSION_ODD => {
                // extension node
                Ok(TrieNode::Extension { prefix: path, node: value })
            }
            PREFIX_LEAF_EVEN | PREFIX_LEAF_ODD => {
                // leaf node
                Ok(TrieNode::Leaf { key: path, value })
            }
            _ => {
                anyhow::bail!("Unexpected path identifier in high-order nibble")
            }
        }
    }
}

impl Decodable for TrieNode {
    /// Attempts to decode the [TrieNode].
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let mut list: VecDeque<_> = Vec::<NodeElement>::decode(buf)?.into();

        match list.len() {
            BRANCH_LIST_LENGTH => Ok(Self::Branch { stack: list }),
            LEAF_OR_EXTENSION_LIST_LENGTH => {
                let Some(NodeElement::String(path)) = list.pop_front() else {
                    return Err(alloy_rlp::Error::UnexpectedList);
                };
                let Some(NodeElement::String(value)) = list.pop_front() else {
                    return Err(alloy_rlp::Error::UnexpectedList);
                };

                Self::try_from_path_and_value(path, value)
                    .map_err(|_| alloy_rlp::Error::UnexpectedList)
            }
            _ => Err(alloy_rlp::Error::UnexpectedLength),
        }
    }
}

/// A [NodeElement] is an element within a MPT node's RLP array
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NodeElement {
    /// An RLP String
    String(Bytes),
    /// An empty RLP string (0x80)
    EmptyString,
    /// An RLP List
    List(VecDeque<Bytes>),
    /// An empty RLP list (0xC0)
    EmptyList,
}

impl NodeElement {
    /// Attempts to convert `Self` into a [TrieNode::Leaf] or [TrieNode::Extension], if `Self` is a
    /// [NodeElement::List] variant.
    pub fn try_list_into_node(self) -> Result<TrieNode> {
        if let NodeElement::List(mut list) = self {
            if list.len() != LEAF_OR_EXTENSION_LIST_LENGTH {
                anyhow::bail!("Invalid length");
            }

            let path = list.pop_front().ok_or(anyhow!("List is empty; Impossible case"))?;
            let value = list.pop_front().ok_or(anyhow!("List is empty; Impossible case"))?;
            TrieNode::try_from_path_and_value(path, value)
        } else {
            anyhow::bail!("Self is not a list")
        }
    }
}

impl Decodable for NodeElement {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        match buf[0] {
            EMPTY_STRING_CODE => {
                buf.advance(1);
                Ok(Self::EmptyString)
            }
            EMPTY_LIST_CODE => {
                buf.advance(1);
                Ok(Self::EmptyList)
            }
            EMPTY_LIST_CODE.. => Ok(Self::List(Vec::<Bytes>::decode(buf)?.into())),
            _ => Ok(Self::String(Bytes::decode(buf)?)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;
    use alloy_primitives::{bytes, hex};

    #[test]
    fn test_decode_branch() {
        const BRANCH_RLP: [u8; 64] = hex!("f83ea0eb08a66a94882454bec899d3e82952dcc918ba4b35a09a84acd98019aef4345080808080808080cd308b8a746573742074687265658080808080808080");
        let expected = TrieNode::Branch {
            stack: vec![
                NodeElement::String(bytes!(
                    "eb08a66a94882454bec899d3e82952dcc918ba4b35a09a84acd98019aef43450"
                )),
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::List(vec![bytes!("30"), bytes!("8a74657374207468726565")].into()),
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
                NodeElement::EmptyString,
            ]
            .into(),
        };
        assert_eq!(expected, TrieNode::decode(&mut BRANCH_RLP.as_slice()).unwrap());
    }

    #[test]
    fn test_decode_extension() {
        const EXTENSION_RLP: [u8; 10] = hex!("c98300646f8476657262");
        let expected = TrieNode::Extension { prefix: bytes!("00646f"), node: bytes!("76657262") };
        assert_eq!(expected, TrieNode::decode(&mut EXTENSION_RLP.as_slice()).unwrap());
    }

    #[test]
    fn test_decode_leaf() {
        const LEAF_RLP: [u8; 11] = hex!("ca8320646f8576657262FF");
        let expected = TrieNode::Leaf { key: bytes!("20646f"), value: bytes!("76657262FF") };
        assert_eq!(expected, TrieNode::decode(&mut LEAF_RLP.as_slice()).unwrap());
    }
}
