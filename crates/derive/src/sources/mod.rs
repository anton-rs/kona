//! The data source module.
//!
//! Data sources are data providers for the kona derivation pipeline.
//! They implement the [DataAvailabilityProvider] trait, providing a way
//! to iterate over data for a given (L2) [BlockInfo].
//!
//! [DataAvailabilityProvider]: crate::traits::DataAvailabilityProvider
//! [BlockInfo]: op_alloy_protocol::BlockInfo

mod blob_data;
pub use blob_data::BlobData;

mod eigenda_data;
pub use eigenda_data::EigenDABlobData;

mod ethereum;
pub use ethereum::EthereumDataSource;

mod eigenda;
pub use eigenda::EigenDADataSource;

mod eigenda_blobs;
pub use eigenda_blobs::EigenDABlobSource;

mod blobs;
pub use blobs::BlobSource;

mod calldata;
pub use calldata::CalldataSource;