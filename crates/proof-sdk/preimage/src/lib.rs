#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/op-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/op-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;

#[macro_use]
extern crate tracing;

pub mod errors;

mod key;
pub use key::{PreimageKey, PreimageKeyType};

mod oracle;
pub use oracle::{OracleReader, OracleServer};

mod hint;
pub use hint::{HintReader, HintWriter};

mod traits;
pub use traits::{
    Channel, CommsClient, HintReaderServer, HintRouter, HintWriterClient, PreimageFetcher,
    PreimageOracleClient, PreimageOracleServer, PreimageServerBackend,
};

#[cfg(any(test, feature = "std"))]
mod native_channel;
#[cfg(any(test, feature = "std"))]
pub use native_channel::{BidirectionalChannel, NativeChannel};
