use crate::PreimageKey;
use alloc::vec::Vec;
use anyhow::Result;

/// A [PreimageOracleClient] is a high-level interface to read data from the host, keyed by a [PreimageKey].
pub trait PreimageOracleClient {
    /// Get the data corresponding to the currently set key from the host. Return the data in a new heap allocated
    /// `Vec<u8>`
    fn get(&mut self, key: PreimageKey) -> Result<Vec<u8>>;

    /// Get the data corresponding to the currently set key from the host. Write the data into the provided buffer
    fn get_exact(&mut self, key: PreimageKey, buf: &mut [u8]) -> Result<()>;
}

/// A [HintWriterClient] is a high-level interface to the hint pipe. It provides a way to write hints to the host.
pub trait HintWriterClient {
    /// Write a hint to the host. This will overwrite any existing hint in the pipe, and block until all data has been
    /// written.
    fn write(&self, hint: &str) -> Result<()>;
}
