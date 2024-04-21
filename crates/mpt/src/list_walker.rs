//! This module contains the [OrderedListWalker] struct, which allows for traversing an MPT root of
//! a derivable ordered list.

use crate::{NodeElement, TrieNode};
use alloc::{collections::VecDeque, vec};
use alloy_primitives::{Bytes, B256};
use alloy_rlp::{Decodable, EMPTY_STRING_CODE};
use anyhow::{anyhow, Result};
use core::{fmt::Display, marker::PhantomData};

/// A [OrderedListWalker] allows for traversing over a Merkle Patricia Trie containing a derivable
/// ordered list.
///
/// Once it has ben hydrated with [Self::hydrate], the elements in the derivable list can be
/// iterated over using the [Iterator] implementation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OrderedListWalker<PreimageFetcher> {
    /// The Merkle Patricia Trie root.
    root: B256,
    /// The leaf nodes of the derived list, in order. [None] if the tree has yet to be fully
    /// traversed with [Self::hydrate].
    inner: Option<VecDeque<Bytes>>,
    /// Phantom data
    _phantom: PhantomData<PreimageFetcher>,
}

impl<PreimageFetcher> OrderedListWalker<PreimageFetcher>
where
    PreimageFetcher: Fn(B256) -> Result<Bytes> + Copy,
{
    /// Creates a new [OrderedListWalker], yet to be hydrated.
    pub fn new(root: B256) -> Self {
        Self { root, inner: None, _phantom: PhantomData }
    }

    /// Creates a new [OrderedListWalker] and hydrates it with [Self::hydrate] and the given fetcher
    /// immediately.
    pub fn try_new_hydrated(root: B256, fetcher: PreimageFetcher) -> Result<Self> {
        let mut walker = Self { root, inner: None, _phantom: PhantomData };
        walker.hydrate(fetcher)?;
        Ok(walker)
    }

    /// Hydrates the [OrderedListWalker]'s iterator with the leaves of the derivable list. If
    /// `Self::inner` is [Some], this function will fail fast.
    pub fn hydrate(&mut self, fetcher: PreimageFetcher) -> Result<()> {
        // Do not allow for re-hydration if `inner` is `Some` and still contains elements.
        if self.inner.is_some() && self.inner.as_ref().map(|s| s.len()).unwrap_or_default() > 0 {
            anyhow::bail!("Iterator is already hydrated, and has not been consumed entirely.")
        }

        // Get the preimage to the root node.
        let root_trie_node = Self::get_trie_node(self.root, fetcher)?;

        // With small lists the iterator seems to use 0x80 (RLP empty string, unlike the others)
        // as key for item 0, causing it to come last. We need to account for this, pulling the
        // first element into its proper position.
        let mut ordered_list = Self::fetch_leaves(root_trie_node, fetcher)?;
        if !ordered_list.is_empty() {
            if ordered_list.len() <= EMPTY_STRING_CODE as usize {
                // If the list length is < 0x80, the final element is the first element.
                let first = ordered_list.pop_back().ok_or(anyhow!("Empty list fetched"))?;
                ordered_list.push_front(first);
            } else {
                // If the list length is > 0x80, the element at index 0x80-1 is the first element.
                let first = ordered_list
                    .remove((EMPTY_STRING_CODE - 1) as usize)
                    .ok_or(anyhow!("Empty list fetched"))?;
                ordered_list.push_front(first);
            }
        }

        self.inner = Some(ordered_list);
        Ok(())
    }

    /// Traverses a [TrieNode], returning all values of child [TrieNode::Leaf] variants.
    fn fetch_leaves(trie_node: TrieNode, fetcher: PreimageFetcher) -> Result<VecDeque<Bytes>> {
        match trie_node {
            TrieNode::Branch { stack } => {
                let mut leaf_values = VecDeque::with_capacity(stack.len());
                for item in stack.into_iter() {
                    match item {
                        NodeElement::String(s) => {
                            // If the string is a hash, we need to grab the preimage for it and
                            // continue recursing.
                            let trie_node = Self::get_trie_node(s.as_ref(), fetcher)?;
                            leaf_values.append(&mut Self::fetch_leaves(trie_node, fetcher)?);
                        }
                        list @ NodeElement::List(_) => {
                            let trie_node = list.try_list_into_node()?;
                            leaf_values.append(&mut Self::fetch_leaves(trie_node, fetcher)?);
                        }
                        _ => { /* Skip over empty lists and strings; We're looking for leaves */ }
                    }
                }
                Ok(leaf_values)
            }
            TrieNode::Leaf { value, .. } => Ok(vec![value].into()),
            TrieNode::Extension { node, .. } => {
                // If the node is a hash, we need to grab the preimage for it and continue
                // recursing.
                let trie_node = Self::get_trie_node(node.as_ref(), fetcher)?;
                Ok(Self::fetch_leaves(trie_node, fetcher)?)
            }
        }
    }

    /// Grabs the preimage of `hash` using `fetcher`, and attempts to decode the preimage data into
    /// a [TrieNode]. Will error if the conversion of `T` into [B256] fails.
    fn get_trie_node<T>(hash: T, fetcher: PreimageFetcher) -> Result<TrieNode>
    where
        T: TryInto<B256>,
        <T as TryInto<B256>>::Error: Display,
    {
        let hash = hash.try_into().map_err(|e| anyhow!("Error in conversion: {e}"))?;
        let preimage = fetcher(hash)?;
        TrieNode::decode(&mut preimage.as_ref()).map_err(|e| anyhow!(e))
    }
}

impl<PreimageFetcher> Iterator for OrderedListWalker<PreimageFetcher> {
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
    use crate::test_util::{get_live_derivable_receipts_list, ordered_trie_with_encoder};
    use alloc::{collections::BTreeMap, string::String, vec::Vec};
    use alloy_consensus::ReceiptEnvelope;
    use alloy_primitives::keccak256;
    use alloy_provider::network::eip2718::Decodable2718;
    use alloy_rlp::Encodable;

    #[tokio::test]
    async fn test_list_walker_online() {
        let (root, preimages, envelopes) = get_live_derivable_receipts_list().await.unwrap();
        let list =
            OrderedListWalker::try_new_hydrated(root, |f| Ok(preimages.get(&f).unwrap().clone()))
                .unwrap();

        assert_eq!(
            list.into_iter()
                .map(|rlp| ReceiptEnvelope::decode_2718(&mut rlp.as_ref()).unwrap())
                .collect::<Vec<_>>(),
            envelopes
        );
    }

    #[test]
    fn test_list_walker() {
        const VALUES: [&str; 3] = ["test one", "test two", "test three"];

        let mut trie = ordered_trie_with_encoder(&VALUES, |v, buf| v.encode(buf));
        let root = trie.root();

        let preimages =
            trie.take_proofs().into_iter().fold(BTreeMap::default(), |mut acc, (_, value)| {
                acc.insert(keccak256(value.as_ref()), value);
                acc
            });

        let list =
            OrderedListWalker::try_new_hydrated(root, |f| Ok(preimages.get(&f).unwrap().clone()))
                .unwrap();

        assert_eq!(
            list.inner
                .unwrap()
                .iter()
                .map(|v| String::decode(&mut v.as_ref()).unwrap())
                .collect::<Vec<_>>(),
            VALUES
        );
    }
}
