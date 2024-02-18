#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

mod key;
pub use key::{PreimageKey, PreimageKeyType};

mod oracle;
pub use oracle::OracleReader;

mod hint;
pub use hint::HintWriter;

mod pipe;
pub use pipe::PipeHandle;

mod traits;
pub use traits::{HintWriterClient, PreimageOracleClient};
