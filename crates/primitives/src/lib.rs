#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/anton-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod blob;
pub use blob::{BlobData, BlobDecodingError, IndexedBlobHash};

pub mod sidecar;
pub use sidecar::{
    APIBlobSidecar, APIConfigResponse, APIGenesisResponse, APIGetBlobSidecarsResponse,
    APIVersionResponse, BeaconBlockHeader, BlobSidecar, SignedBeaconBlockHeader,
    VersionInformation, KZG_COMMITMENT_SIZE, KZG_PROOF_SIZE,
};
