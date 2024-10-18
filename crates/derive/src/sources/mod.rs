//! This module contains data source impelmentations.

mod ethereum;
pub use ethereum::EthereumDataSource;

mod blobs;
pub use blobs::{BlobData, BlobSource, IndexedBlobHash};

mod calldata;
pub use calldata::CalldataSource;

mod variant;
pub use variant::EthereumDataSourceVariant;
