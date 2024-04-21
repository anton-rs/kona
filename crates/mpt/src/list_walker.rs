//! This module contains the [OrderedListWalker] struct, which allows for traversing an MPT root of
//! a derivable ordered list.

use crate::{NodeElement, TrieNode};
use alloc::{collections::VecDeque, vec};
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use anyhow::{anyhow, Result};

/// A [OrderedListWalker] allows for traversing over a Merkle Patricia Trie containing a derivable
/// ordered list.
///
/// Once it has ben hydrated with [Self::hydrate], the elements in the derivable list can be
/// iterated over using the [Iterator] implementation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OrderedListWalker {
    /// The Merkle Patricia Trie root.
    root: B256,
    /// The leaf nodes of the derived list, in order. [None] if the tree has yet to be fully
    /// traversed with [Self::hydrate].
    inner: Option<VecDeque<Bytes>>,
}

impl OrderedListWalker {
    /// Creates a new [DerivableListWalker].
    pub fn new(root: B256) -> Self {
        Self { root, inner: None }
    }

    /// Hydrates the [DerivableListWalker]'s iterator with the leaves of the derivable list. If
    /// [Self::inner] is [Some], this function will fail fast.
    pub fn hydrate(&mut self, fetcher: impl Fn(B256) -> Result<Bytes> + Copy) -> Result<()> {
        if self.inner.is_some() && self.inner.as_ref().map(|s| s.len()).unwrap_or_default() > 0 {
            anyhow::bail!("Iterator is already hydrated, and has not been consumed entirely.")
        }

        let root_trie_node = Self::get_trie_node(self.root, fetcher)?;
        self.inner = Some(Self::hydrate_trie_node(root_trie_node, fetcher)?);
        Ok(())
    }

    /// Traverses a [TrieNode], returning all values of child [TrieNode::Leaf] variants.
    fn hydrate_trie_node(
        trie_node: TrieNode,
        fetcher: impl Fn(B256) -> Result<Bytes> + Copy,
    ) -> Result<VecDeque<Bytes>> {
        match trie_node {
            TrieNode::Branch { stack } => {
                let mut leaf_values = VecDeque::with_capacity(stack.len());
                for item in stack.into_iter() {
                    match item {
                        NodeElement::String(s) => {
                            // If the string is a hash, we need to grab the preimage for it and
                            // continue recursing.
                            if s.len() == B256::len_bytes() {
                                let hash = B256::from_slice(s.as_ref());
                                let trie_node = Self::get_trie_node(hash, fetcher)?;
                                leaf_values
                                    .append(&mut Self::hydrate_trie_node(trie_node, fetcher)?);
                            } else {
                                anyhow::bail!("Unexpected string in branch node: {s}");
                            }
                        }
                        s @ NodeElement::List(_) => {
                            let trie_node = s.try_list_into_node()?;
                            leaf_values.append(&mut Self::hydrate_trie_node(trie_node, fetcher)?);
                        }
                        _ => { /* Skip over empty lists and strings */ }
                    }
                }
                Ok(leaf_values)
            }
            TrieNode::Leaf { value, .. } => Ok(vec![value].into()),
            TrieNode::Extension { .. } => {
                unreachable!("No extension nodes within Derivable Lists")
            }
        }
    }

    /// Grabs the preimage of `hash` using `fetcher`, and attempts to decode the preimage data into
    /// a [TrieNode].
    fn get_trie_node(hash: B256, fetcher: impl Fn(B256) -> Result<Bytes>) -> Result<TrieNode> {
        let preimage = fetcher(hash)?;
        TrieNode::decode(&mut preimage.as_ref()).map_err(|e| anyhow!(e))
    }
}

impl Iterator for OrderedListWalker {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            Some(ref mut leaves) => leaves.pop_front(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use crate::test_util::construct_derivable_list;
    use alloc::vec::Vec;
    use alloy_primitives::{b256, bytes};
    use alloy_rlp::Encodable;
    use std::collections::HashMap;

    const VALUES: [&str; 3] = ["test one", "test two", "test three"];

    #[test]
    fn test_list_walker() {
        let rlp_values = VALUES
            .iter()
            .map(|v| {
                let mut rlp_buf = Vec::with_capacity(v.length());
                v.encode(&mut rlp_buf);
                rlp_buf.into()
            })
            .collect::<Vec<_>>();
        let mut trie = construct_derivable_list(&rlp_values);
        let root = trie.root();

        let mut preimages: HashMap<B256, Bytes> = HashMap::new();
        preimages.insert(b256!("460a04e80ab66fcd2c5ff3def90e4f19be55d2bcbf186901b30d9fef201bbc2a"), bytes!("f83ea0eb08a66a94882454bec899d3e82952dcc918ba4b35a09a84acd98019aef4345080808080808080cd308b8a746573742074687265658080808080808080"));
        preimages.insert(
            b256!("eb08a66a94882454bec899d3e82952dcc918ba4b35a09a84acd98019aef43450"),
            bytes!(
                "e780cb20898874657374206f6e65cb208988746573742074776f8080808080808080808080808080"
            ),
        );

        let mut list = OrderedListWalker::new(root);
        list.hydrate(|f| Ok(preimages.get(&f).unwrap().clone())).unwrap();

        assert_eq!(list.inner.unwrap(), rlp_values);
    }
}
