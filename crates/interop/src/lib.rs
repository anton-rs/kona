#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod graph;
pub use graph::DependencyGraph;

mod message;
pub use message::{ExecutingMessage, MessageIdentifier, RawMessagePayload};

mod constants;
pub use constants::{
    CROSS_L2_INBOX_ADDRESS, MESSAGE_EXPIRY_WINDOW, SUPER_ROOT_VERSION, TRANSITION_STATE_VERSION,
};

mod traits;
pub use traits::InteropProvider;

mod errors;
pub use errors::{
    DependencyGraphError, DependencyGraphResult, InteropProviderError, InteropProviderResult,
};

mod pre_state;
pub use pre_state::{OutputRootWithBlockHash, OutputRootWithChain, SuperRoot, TransitionState};

#[cfg(test)]
mod test_utils;
