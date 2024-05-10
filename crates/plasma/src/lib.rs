#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

pub mod plasma;
pub mod source;
pub mod traits;
pub mod types;

#[cfg(test)]
pub mod test_utils;
