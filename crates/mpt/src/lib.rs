#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

mod db;
pub use db::{TrieAccount, TrieDB};

mod fetcher;
pub use fetcher::{NoopTrieDBFetcher, NoopTrieDBHinter, TrieDBFetcher, TrieDBHinter};

mod node;
pub use node::TrieNode;

mod list_walker;
pub use list_walker::OrderedListWalker;

mod util;
pub use util::ordered_trie_with_encoder;

#[cfg(test)]
mod test_util;
