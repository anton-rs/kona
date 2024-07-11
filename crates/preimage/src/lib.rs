#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

mod key;
pub use key::{PreimageKey, PreimageKeyType};

mod traits;
pub use traits::{
    HintReaderServer, HintRouter, HintWriterClient, PreimageFetcher, PreimageOracleClient,
    PreimageOracleServer,
};

cfg_if::cfg_if! {
    if #[cfg(not(feature = "no-io"))] {
        mod oracle;
        pub use oracle::{OracleReader, OracleServer};

        mod hint;
        pub use hint::{HintReader, HintWriter};

        mod pipe;
        pub use pipe::PipeHandle;
    }
}
