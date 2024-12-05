#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[macro_use]
extern crate tracing;

mod errors;
pub use errors::{ExecutorError, ExecutorResult, TrieDBError, TrieDBResult};

mod executor;
pub use executor::{
    KonaHandleRegister, StatelessL2BlockExecutor, StatelessL2BlockExecutorBuilder, KonaEvmConfig
};

mod db;
pub use db::{NoopTrieDBProvider, TrieAccount, TrieDB, TrieDBProvider};

mod constants;
mod syscalls;
