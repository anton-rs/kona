//! This module contains data source impelmentations.

mod errors;
pub use errors::{BlobDecodingError, BlobProviderError};

mod ethereum;
pub use ethereum::EthereumDataSource;

mod blobs;
pub use blobs::{BlobData, BlobSource, IndexedBlobHash};

mod calldata;
pub use calldata::CalldataSource;

mod variant;
pub use variant::EthereumDataSourceVariant;
