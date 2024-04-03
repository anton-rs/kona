use crate::PreimageKey;
use alloc::vec::Vec;
use anyhow::Result;

/// A [PreimageOracleClient] is a high-level interface to read data from the host, keyed by a
/// [PreimageKey].
pub trait PreimageOracleClient {
    /// Get the data corresponding to the currently set key from the host. Return the data in a new
    /// heap allocated `Vec<u8>`
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)` if the data was successfully fetched from the host.
    /// - `Err(_)` if the data could not be fetched from the host.
    fn get(&mut self, key: PreimageKey) -> Result<Vec<u8>>;

    /// Get the data corresponding to the currently set key from the host. Writes the data into the
    /// provided buffer.
    ///
    /// # Returns
    /// - `Ok(())` if the data was successfully written into the buffer.
    /// - `Err(_)` if the data could not be written into the buffer.
    fn get_exact(&mut self, key: PreimageKey, buf: &mut [u8]) -> Result<()>;
}

/// A [HintWriterClient] is a high-level interface to the hint pipe. It provides a way to write
/// hints to the host.
pub trait HintWriterClient {
    /// Write a hint to the host. This will overwrite any existing hint in the pipe, and block until
    /// all data has been written.
    ///
    /// # Returns
    /// - `Ok(())` if the hint was successfully written to the host.
    /// - `Err(_)` if the hint could not be written to the host.
    fn write(&self, hint: &str) -> Result<()>;
}
