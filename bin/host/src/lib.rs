#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod orchestrator;
pub use orchestrator::{DetachedHostOrchestrator, HostOrchestrator};
