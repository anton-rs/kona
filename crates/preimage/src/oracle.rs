use crate::{PipeHandle, PreimageKey};
use alloc::vec::Vec;
use anyhow::{bail, Result};

/// An [OracleReader] is a high-level interface to the preimage oracle.
#[derive(Debug, Clone, Copy)]
pub struct OracleReader {
    pipe_handle: PipeHandle,
}

impl OracleReader {
    /// Create a new [OracleReader] from a [PipeHandle].
    pub fn new(pipe_handle: PipeHandle) -> Self {
        Self { pipe_handle }
    }

    /// Get the data corresponding to the currently set key from the host. Return the data in a new heap allocated
    /// `Vec<u8>`
    pub fn get(&mut self, key: PreimageKey) -> Result<Vec<u8>> {
        let length = self.write_key(key)?;
        let mut data_buffer = alloc::vec![0; length];

        // Grab a read lock on the preimage pipe to read the data.
        self.pipe_handle.read_exact(&mut data_buffer)? as usize;

        Ok(data_buffer)
    }

    /// Get the data corresponding to the currently set key from the host. Write the data into the provided buffer
    pub fn get_exact(&mut self, key: PreimageKey, buf: &mut [u8]) -> Result<()> {
        // Write the key to the host and read the length of the preimage.
        let length = self.write_key(key)?;

        // Ensure the buffer is the correct size.
        if buf.len() != length {
            bail!(
                "Buffer size {} does not match preimage size {}",
                buf.len(),
                length
            );
        }

        self.pipe_handle.read_exact(buf)?;

        Ok(())
    }

    /// Set the preimage key for the global oracle reader. This will overwrite any existing key, and block until the 
    /// host has prepared the preimage and responded with the length of the preimage.
    fn write_key(&mut self, key: PreimageKey) -> Result<usize> {
        // Write the key to the host so that it can prepare the preimage.
        let key_bytes: [u8; 32] = key.into();
        self.pipe_handle.write(&key_bytes)?;

        // Read the length prefix and reset the cursor.
        let mut length_buffer = [0u8; 8];
        self.pipe_handle.read_exact(&mut length_buffer)?;
        Ok(u64::from_be_bytes(length_buffer) as usize)
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use crate::PreimageKeyType;
    use kona_common::FileDescriptor;
    use std::{
        borrow::ToOwned,
        fs::{File, OpenOptions},
        os::fd::AsRawFd,
    };

    /// Test struct containing the [OracleReader] and a [PipeHandle] for the host, plus the open [File]s. The [File]s
    /// are stored in this struct so that they are not dropped until the end of the test.
    ///
    /// TODO: Swap host pipe handle to oracle writer once it exists.
    #[derive(Debug)]
    struct ClientAndHost {
        oracle_reader: OracleReader,
        host_handle: PipeHandle,
        _read_file: File,
        _write_file: File,
    }

    impl Drop for ClientAndHost {
        fn drop(&mut self) {
            std::fs::remove_file("/tmp/read.hex").unwrap();
            std::fs::remove_file("/tmp/write.hex").unwrap();
        }
    }

    /// Helper for opening a file with the correct options.
    fn open_options() -> OpenOptions {
        File::options()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .to_owned()
    }

    /// Helper for creating a new [OracleReader] and [PipeHandle] for testing. The file channel is over two temporary
    /// files.
    ///
    /// TODO: Swap host pipe handle to oracle writer once it exists.
    fn client_and_host() -> ClientAndHost {
        let (read_file, write_file) = (
            open_options().open("/tmp/read.hex").unwrap(),
            open_options().open("/tmp/write.hex").unwrap(),
        );
        let (read_fd, write_fd) = (
            FileDescriptor::Wildcard(read_file.as_raw_fd().try_into().unwrap()),
            FileDescriptor::Wildcard(write_file.as_raw_fd().try_into().unwrap()),
        );
        let client_handle = PipeHandle::new(read_fd, write_fd);
        let host_handle = PipeHandle::new(write_fd, read_fd);

        let oracle_reader = OracleReader::new(client_handle);

        ClientAndHost {
            oracle_reader,
            host_handle,
            _read_file: read_file,
            _write_file: write_file,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_oracle_reader() {
        const MOCK_DATA: &[u8] = b"1234567890";
        let sys = client_and_host();
        let (mut oracle_reader, host_handle) = (sys.oracle_reader, sys.host_handle);

        let client = tokio::task::spawn(async move {
            let mut buf = [0u8; 10];
            oracle_reader
                .get_exact(
                    PreimageKey::new([0u8; 32], PreimageKeyType::Keccak256),
                    &mut buf,
                )
                .unwrap();
            buf
        });
        let host = tokio::task::spawn(async move {
            let mut length_and_data: [u8; 8 + 10] = [0u8; 8 + 10];
            length_and_data[0..8].copy_from_slice(&u64::to_be_bytes(MOCK_DATA.len() as u64));
            length_and_data[8..18].copy_from_slice(MOCK_DATA);
            host_handle.write(&length_and_data).unwrap();
        });

        let (r, _) = tokio::join!(client, host);
        assert_eq!(r.unwrap(), MOCK_DATA);
    }
}
