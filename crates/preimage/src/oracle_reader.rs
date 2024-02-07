use crate::{PipeHandle, PreimageKey};
use alloc::vec::Vec;
use anyhow::Result;
use kona_common::io::FileDescriptor;
use spin::RwLock;

/// An [OracleReader] is a high-level interface to the preimage oracle.
#[derive(Debug)]
pub struct OracleReader {
    key: Option<PreimageKey>,
    pipe_handle: PipeHandle,
    length: usize,
    cursor: usize,
}

/// The only way to access an oracle reader is through this singleton. This is to ensure there cannot be more than one
/// at a time, which would have undefined behavior.
pub static ORACLE_READER: RwLock<OracleReader> = RwLock::new(OracleReader {
    key: None,
    pipe_handle: PipeHandle::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite),
    length: 0,
    cursor: 0,
});

impl OracleReader {
    /// Return the current key stored in the global oracle reader
    pub fn key(&self) -> Option<PreimageKey> {
        self.key
    }

    /// length of the current pre-image
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
    /// let data = oracle.get(key).unwrap();
    /// ```
    pub fn get(&mut self, key: PreimageKey) -> Result<Vec<u8>> {
        self.set_key(key)?;
        let mut data_buffer = alloc::vec![0; self.length];
        self.cursor += self.pipe_handle.read(&mut data_buffer)? as usize;
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
    /// oracle.get_exact(key, &mut buffer).unwrap();
    /// ```
    pub fn get_exact(&mut self, key: PreimageKey, buf: &mut [u8]) -> Result<()> {
        self.set_key(key)?;
        assert!(self.length == buf.len(), "Buffer not correct size for preimage data. Preimage size: {} bytes, buffer size: {} bytes", self.length, buf.len());
        self.cursor += self.pipe_handle.read(buf)? as usize;
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
        self.pipe_handle.write(&key_bytes)?;

        // Read the length prefix and reset the cursor.
        let mut length_buffer = [0u8; 8];
        self.pipe_handle.read(&mut length_buffer)?;
        self.length = u64::from_be_bytes(length_buffer) as usize;
        self.cursor = 0;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::PreimageKeyType;
    use std::{
        fs::File,
        io::Write,
        os::fd::{AsRawFd, FromRawFd},
    };

    extern crate std;

    fn create_file_at_fd(path: &str, target_fd: i32) -> std::io::Result<()> {
        // Open the file normally. This gets us a File with some arbitrary FD.
        let file = File::create(path)?;

        // Extract the raw file descriptor from the File object.
        let original_fd = file.as_raw_fd();

        // Use dup2 to duplicate the file descriptor to the target FD.
        // SAFETY: This operation is unsafe because it can affect global process state and may cause
        // race conditions or security issues if not used carefully.
        unsafe {
            let dup_result = libc::dup2(original_fd, target_fd);
            if dup_result == -1 {
                // If dup2 failed, return an error.
                return Err(std::io::Error::last_os_error());
            }
        }

        // At this point, the file is duplicated to `target_fd`. The original FD is still open.
        // We close the original to avoid resource leaks.
        // Dropping the original File object will not close the new FD at `target_fd`.
        drop(file);

        // SAFETY: We now safely wrap the target FD back into a File.
        // The caller must ensure that `target_fd` is not accessed concurrently by other parts of the program.
        let file = unsafe { File::from_raw_fd(target_fd) };
        std::mem::forget(file);

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_oracle_reader() {
        create_file_at_fd(
            "/tmp/preimage-read.dat",
            FileDescriptor::PreimageRead as i32,
        )
        .unwrap();
        create_file_at_fd(
            "/tmp/preimage-write.dat",
            FileDescriptor::PreimageWrite as i32,
        )
        .unwrap();

        let mut read = unsafe { File::from_raw_fd(FileDescriptor::PreimageRead as i32) };
        let mut data = [0u8; 40];
        data[7] = 0x20;
        read.write_all(data.as_ref()).unwrap();

        ORACLE_READER
            .write()
            .get(PreimageKey::new([0u8; 32], PreimageKeyType::Local))
            .unwrap();
    }
}
