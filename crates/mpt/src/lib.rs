#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod db;
pub use db::{TrieAccount, TrieDB};

mod errors;
pub use errors::{
    OrderedListWalkerError, OrderedListWalkerResult, TrieDBError, TrieDBResult, TrieNodeError,
    TrieNodeResult,
};

mod fetcher;
pub use fetcher::{NoopTrieHinter, NoopTrieProvider, TrieHinter, TrieProvider};

mod node;
pub use node::TrieNode;

mod list_walker;
pub use list_walker::OrderedListWalker;

mod util;
pub use util::ordered_trie_with_encoder;

#[cfg(test)]
mod test_util;
