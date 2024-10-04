#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![no_std]

extern crate alloc;

/// Re-export commonly used types and traits.
pub mod prelude {
    pub use super::*;
}

mod l1_chain_provider;
pub use l1_chain_provider::ChainProvider;

mod l2_chain_provider;
pub use l2_chain_provider::L2ChainProvider;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
