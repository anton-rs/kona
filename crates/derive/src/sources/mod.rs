//! This module contains data source impelmentations.

mod ethereum;
pub use ethereum::EthereumDataSource;

mod blobs;
pub use blobs::BlobSource;

mod calldata;
pub use calldata::CalldataSource;

mod source;
pub use source::EthereumDataSourceVariant;
