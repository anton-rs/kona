#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

mod key;
pub use key::{PreimageKey, PreimageKeyType};

mod oracle;
pub use oracle::{OracleReader, OracleServer};

mod hint;
pub use hint::{HintReader, HintWriter};

mod pipe;
pub use pipe::PipeHandle;

mod traits;
pub use traits::{HintReaderServer, HintWriterClient, PreimageOracleClient, PreimageOracleServer};
