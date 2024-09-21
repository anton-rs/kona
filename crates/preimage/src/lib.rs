#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![no_std]

extern crate alloc;

pub mod errors;

mod key;
pub use key::{PreimageKey, PreimageKeyType};

mod oracle;
pub use oracle::{OracleReader, OracleServer};

mod hint;
pub use hint::{HintReader, HintWriter};

mod pipe;
pub use pipe::PipeHandle;

mod traits;
pub use traits::{
    CommsClient, HintReaderServer, HintRouter, HintWriterClient, PreimageFetcher,
    PreimageOracleClient, PreimageOracleServer,
};

#[cfg(test)]
mod test_utils;
