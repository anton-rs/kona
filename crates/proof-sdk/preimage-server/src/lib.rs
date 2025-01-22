#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/op-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/op-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod fetcher;
pub use fetcher::Fetcher;

mod kv;
pub use kv::{
    DiskKeyValueStore, KeyValueStore, MemoryKeyValueStore, SharedKeyValueStore, SplitKeyValueStore,
};

mod preimage;
pub use preimage::{
    OfflineHintRouter, OfflinePreimageFetcher, OnlineHintRouter, OnlinePreimageFetcher,
};

mod server;
pub use server::PreimageServer;
