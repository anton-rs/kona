//! This module contains data source impelmentations.

mod factory;
pub use factory::DataSourceFactory;

mod blobs;
pub use blobs::BlobSource;

mod calldata;
pub use calldata::CalldataSource;

mod plasma;
pub use plasma::PlasmaSource;

mod source;
pub use source::DataSource;
