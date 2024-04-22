#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

mod node;
pub use node::{NodeElement, TrieNode};

mod list_walker;
pub use list_walker::OrderedListWalker;

#[cfg(test)]
mod test_util;
