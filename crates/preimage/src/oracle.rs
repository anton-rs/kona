use crate::{PipeHandle, PreimageKey};
use alloc::vec::Vec;
use anyhow::{bail, Result};

/// An [OracleReader] is a high-level interface to the preimage oracle.
#[derive(Debug, Clone, Copy)]
pub struct OracleReader {
    pipe_handle: PipeHandle,
    key: Option<PreimageKey>,
    length: usize,
    cursor: usize,
}

impl OracleReader {
    /// Create a new [OracleReader] from a [PipeHandle].
    pub fn new(pipe_handle: PipeHandle) -> Self {
        Self {
            pipe_handle,
            key: None,
            length: 0,
            cursor: 0,
        }
    }

    /// Return the current key stored in the global oracle reader
    pub fn key(&self) -> Option<PreimageKey> {
        self.key
    }

    /// Return the length of the current pre-image
    pub fn length(&self) -> usize {
        self.length
    }

    /// Current position of the read cursor within the current pre-image
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Get the data corresponding to the currently set key from the host. Return the data in a new heap allocated `Vec<u8>`
    ///
    /// Internally this reads self.length bytes from the ReadPreimage file descriptor into a new heap allocated `Vec<u8>` and returns it.
    /// This is a high level way to interact with the preimage oracle but may not be the best way if heap allocations are not desirable.
    pub fn get(&mut self, key: PreimageKey) -> Result<Vec<u8>> {
        self.set_key(key)?;
        let mut data_buffer = alloc::vec![0; self.length];

        // Grab a read lock on the preimage pipe to read the data.
        self.read_exact(&mut data_buffer)?;

        Ok(data_buffer)
    }

    /// Get the data corresponding to the currently set key from the host. Write the data into the provided buffer
    ///
    /// # Panics
    /// This will panic if the size of the buffer is not equal to the size of the preimage as reported by the host
    pub fn get_exact(&mut self, key: PreimageKey, buf: &mut [u8]) -> Result<()> {
        self.set_key(key)?;

        // Ensure the buffer is the correct size.
        if buf.len() != self.length {
            bail!(
                "Buffer size {} does not match preimage size {}",
                buf.len(),
                self.length
            );
        }

        // Grab a read lock on the preimage pipe to read the data.
        self.read_exact(buf)?;

        Ok(())
    }

    /// Set the preimage key for the global oracle reader. This will overwrite any existing key, and block until all
    /// data has been read from the host.
    ///
    /// Internally this sends the 32 bytes of the key to the host by writing into the WritePreimage file descriptor.
    /// This may require several writes as the host may only accept a few bytes at a time. Once 32 bytes have been written
    /// successfully the key is considered set. If it fails to write 32 bytes it will return an error.
    /// Once it has written the key it will read the first 8 bytes of the ReadPreimage file descriptor which is the length
    /// encoded as a big endian u64. This is stored in the oracle reader along with the read cursor position.
    fn set_key(&mut self, key: PreimageKey) -> Result<()> {
        // Set the active key.
        self.key = Some(key);

        // Write the key to the host so that it can prepare the preimage.
        let key_bytes: [u8; 32] = key.into();
        let mut written = 0;
        loop {
            match self.pipe_handle.write(&key_bytes[written..]) {
                Ok(0) => break,
                Ok(n) => {
                    written += n as usize;
                    continue;
                }
                Err(e) => bail!("Failed to write preimage key: {}", e),
            }
        }

        // Read the length prefix and reset the cursor.
        let mut length_buffer = [0u8; 8];
        self.read_exact(&mut length_buffer)?;
        self.length = u64::from_be_bytes(length_buffer) as usize;
        self.cursor = 0;
        Ok(())
    }

    /// Reads bytes into `buf` and returns the number of bytes read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let read = self.pipe_handle.read(buf)?;
        self.cursor += read as usize;
        Ok(read as usize)
    }

    /// Reads exactly `buf.len()` bytes into `buf`, blocking until all bytes are read.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut read = 0;
        while read < buf.len() {
            let chunk_read = self.read(&mut buf[read..])?;
            read += chunk_read;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use crate::{PreimageKeyType, ReadHandle, WriteHandle};
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
        let client_handle = PipeHandle::new(ReadHandle::new(read_fd), WriteHandle::new(write_fd));
        let host_handle = PipeHandle::new(ReadHandle::new(write_fd), WriteHandle::new(read_fd));

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
