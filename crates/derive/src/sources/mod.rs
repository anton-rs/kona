//! The data source module.
//!
//! Data sources are data providers for the kona derivation pipeline.
//! They implement the [DataAvailabilityProvider] trait, providing a way
//! to iterate over data for a given (L2) [BlockInfo].
//!
//! [DataAvailabilityProvider]: crate::traits::DataAvailabilityProvider
//! [BlockInfo]: op_alloy_protocol::BlockInfo

mod ethereum;
pub use ethereum::EthereumDataSource;

mod blobs;
pub use blobs::{BlobData, BlobSource, IndexedBlobHash};

mod calldata;
pub use calldata::CalldataSource;

mod variant;
pub use variant::EthereumDataSourceVariant;
