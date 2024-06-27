use crate::PreimageKey;
use alloc::{boxed::Box, string::String, vec::Vec};
use anyhow::Result;
use async_trait::async_trait;
use core::future::Future;

/// A [PreimageOracleClient] is a high-level interface to read data from the host, keyed by a
/// [PreimageKey].
#[async_trait]
pub trait PreimageOracleClient {
    /// Get the data corresponding to the currently set key from the host. Return the data in a new
    /// heap allocated `Vec<u8>`
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)` if the data was successfully fetched from the host.
    /// - `Err(_)` if the data could not be fetched from the host.
    async fn get(&self, key: PreimageKey) -> Result<Vec<u8>>;

    /// Get the data corresponding to the currently set key from the host. Writes the data into the
    /// provided buffer.
    ///
    /// # Returns
    /// - `Ok(())` if the data was successfully written into the buffer.
    /// - `Err(_)` if the data could not be written into the buffer.
    async fn get_exact(&self, key: PreimageKey, buf: &mut [u8]) -> Result<()>;
}

/// A [HintWriterClient] is a high-level interface to the hint pipe. It provides a way to write
/// hints to the host.
#[async_trait]
pub trait HintWriterClient {
    /// Write a hint to the host. This will overwrite any existing hint in the pipe, and block until
    /// all data has been written.
    ///
    /// # Returns
    /// - `Ok(())` if the hint was successfully written to the host.
    /// - `Err(_)` if the hint could not be written to the host.
    async fn write(&self, hint: &str) -> Result<()>;
}

/// A [CommsClient] is a trait that combines the [PreimageOracleClient] and [HintWriterClient]
pub trait CommsClient: PreimageOracleClient + Clone + HintWriterClient {}

// Implement the super trait for any type that satisfies the bounds
impl<T: PreimageOracleClient + Clone + HintWriterClient> CommsClient for T {}

/// A [PreimageOracleServer] is a high-level interface to accept read requests from the client and
/// write the preimage data to the client pipe.
#[async_trait]
pub trait PreimageOracleServer {
    /// Get the next preimage request and return the response to the client.
    ///
    /// # Returns
    /// - `Ok(())` if the data was successfully written into the client pipe.
    /// - `Err(_)` if the data could not be written to the client.
    async fn next_preimage_request<F, Fut>(&self, get_preimage: F) -> Result<()>
    where
        F: FnMut(PreimageKey) -> Fut + Send,
        Fut: Future<Output = Result<Vec<u8>>> + Send;
}

/// A [HintReaderServer] is a high-level interface to read preimage hints from the
/// [HintWriterClient] and prepare them for consumption by the client program.
#[async_trait]
pub trait HintReaderServer {
    /// Get the next hint request and return the acknowledgement to the client.
    ///
    /// # Returns
    /// - `Ok(())` if the hint was received and the client was notified of the host's
    ///   acknowledgement.
    /// - `Err(_)` if the hint was not received correctly.
    async fn next_hint<F, Fut>(&self, route_hint: F) -> Result<()>
    where
        F: FnMut(String) -> Fut + Send,
        Fut: Future<Output = Result<()>> + Send;
}
