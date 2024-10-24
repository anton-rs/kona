#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(test), no_std)]

extern crate alloc;
extern crate noerror as thiserror;

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
