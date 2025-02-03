//! This module contains the super-chain (interop) mode for the host.

mod cfg;
pub use cfg::{InteropHost, InteropProviders};

mod local_kv;
pub use local_kv::InteropLocalInputs;

mod fetcher;
pub use fetcher::InteropFetcher;
