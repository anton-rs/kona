use crate::{
    pipe::{ReadHandle, WriteHandle},
    PipeHandle, PreimageKey,
};
use alloc::vec::Vec;
use anyhow::{bail, Result};
use kona_common::io::FileDescriptor;
use spin::RwLock;

/// An [OracleReader] is a high-level interface to the preimage oracle.
#[derive(Debug)]
pub struct OracleReader<'pipe> {
    pipe_handle: PipeHandle<'pipe>,
    key: Option<PreimageKey>,
    length: usize,
    cursor: usize,
}

static PREIMAGE_READ_LOCK: RwLock<FileDescriptor> = RwLock::new(FileDescriptor::PreimageRead);
static PREIMAGE_WRITE_LOCK: RwLock<FileDescriptor> = RwLock::new(FileDescriptor::PreimageWrite);

/// The preimage pipe is a bidirectional pipe that is used to communicate preimage requests and responses between the
/// host and the client. There can only be one client preimage pipe handle at a time, so it is a static singleton.
static mut CLIENT_PREIMAGE_PIPE_HANDLE: Option<PipeHandle<'static>> = Some(PipeHandle::new(
    ReadHandle::new(&PREIMAGE_READ_LOCK),
    WriteHandle::new(&PREIMAGE_WRITE_LOCK),
));

/// Fetch the global [PipeHandle] for the client preimage channel.
///
/// # Panics
/// Panics if ownership over the global [PipeHandle] has already been taken.
pub fn client_preimage_pipe_handle() -> PipeHandle<'static> {
    unsafe {
        let reader = CLIENT_PREIMAGE_PIPE_HANDLE.take();
        reader.expect("Client preimage pipe handle already in use")
    }
}

impl<'pipe> OracleReader<'pipe> {
    /// Create a new [OracleReader] from a [PipeHandle].
    pub fn new(pipe_handle: PipeHandle<'pipe>) -> Self {
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
    ///
    /// # Examples
    /// ```
    /// use kona_preimage::{oracle_reader, PreimageKey, PreimageKeyType};
    ///
    /// let mut oracle = oracle_reader();
    /// let key = PreimageKey::new([0u8; 32], PreimageKeyType::Local);
    /// // let data = oracle.get(key).unwrap();
    /// ```
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
    ///
    /// # Examples
    /// ```
    /// use kona_preimage::{oracle_reader, PreimageKey, PreimageKeyType};
    ///
    /// let mut oracle = oracle_reader();
    /// let key = PreimageKey::new([0u8; 32], PreimageKeyType::Local);
    /// let mut buffer = [0_u8; 100];
    /// // oracle.get_exact(key, &mut buffer).unwrap();
    /// ```
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

    /// Set the preimage key for the global oracle reader. This will overwrite any existing key
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

    /// Reads exactly `buf.len()` bytes into `buf`.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut chunk = [0u8; 32];
        let mut read = 0;
        while read < buf.len() {
            let chunk_read = self.read(&mut chunk)?;
            if chunk_read == 0 {
                bail!("Failed to read preimage");
            }
            buf[read..(read + chunk_read)].copy_from_slice(&chunk[..chunk_read]);
            read += chunk_read;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use std::{
        borrow::ToOwned,
        fs::{File, OpenOptions},
        io::{Read, Write},
        os::fd::AsRawFd,
    };

    /// Helper for opening a file with the correct options.
    fn open_options() -> OpenOptions {
        File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .to_owned()
    }

    #[test]
    #[ignore]
    fn test_oracle_reader() {
        const MOCK_DATA: &[u8] = b"1234567890";

        let (mut read, mut write) = (
            open_options().open("/tmp/read.hex").unwrap(),
            open_options().open("/tmp/write.hex").unwrap(),
        );
        let (rwlock_a, rwlock_b) = (
            RwLock::new(FileDescriptor::Wildcard(
                read.as_raw_fd().try_into().unwrap(),
            )),
            RwLock::new(FileDescriptor::Wildcard(
                write.as_raw_fd().try_into().unwrap(),
            )),
        );
        let pipe_handle = PipeHandle::new(ReadHandle::new(&rwlock_a), WriteHandle::new(&rwlock_b));

        let mut buf = [0u8; 10];

        // Ensure writing to the pipe works.
        pipe_handle.write(MOCK_DATA).unwrap();
        write.read(&mut buf).unwrap();
        assert_eq!(buf, MOCK_DATA);

        // Write mock data to the read end of the pipe; There's no host to respond.
        read.write_all(MOCK_DATA).unwrap();
        // Ensure reading from the pipe works.
        pipe_handle.read(&mut buf).unwrap();
        assert_eq!(buf, MOCK_DATA);

        drop(read);
        drop(write);
    }
}
