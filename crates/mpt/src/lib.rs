#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
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
