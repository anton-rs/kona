use crate::{
    traits::{HintRouter, HintWriterClient},
    HintReaderServer, PipeHandle,
};
use alloc::{boxed::Box, string::String, vec};
use anyhow::Result;
use async_trait::async_trait;
use tracing::{error, trace};

/// A [HintWriter] is a high-level interface to the hint pipe. It provides a way to write hints to
/// the host.
#[derive(Debug, Clone, Copy)]
pub struct HintWriter {
    #[cfg(not(feature = "solo-client"))]
    pipe_handle: PipeHandle,
}

impl HintWriter {
    /// Create a new [HintWriter] from a [PipeHandle].
    #[cfg(not(feature = "solo-client"))]
    pub const fn new(pipe_handle: PipeHandle) -> Self {
        Self { pipe_handle }
    }

    /// Create a new [HintWriter] from a [PipeHandle].
    #[cfg(feature = "solo-client")]
    pub const fn new(_: PipeHandle) -> Self {
        Self {}
    }
}

#[async_trait]
impl HintWriterClient for HintWriter {
    /// Write a hint to the host. This will overwrite any existing hint in the pipe, and block until
    /// all data has been written.
    #[cfg(not(feature = "solo-client"))]
    async fn write(&self, hint: &str) -> Result<()> {
        // Form the hint into a byte buffer. The format is a 4-byte big-endian length prefix
        // followed by the hint string.
        let mut hint_bytes = vec![0u8; hint.len() + 4];
        hint_bytes[0..4].copy_from_slice(u32::to_be_bytes(hint.len() as u32).as_ref());
        hint_bytes[4..].copy_from_slice(hint.as_bytes());

        trace!(target: "hint_writer", "Writing hint \"{hint}\"");

        // Write the hint to the host.
        self.pipe_handle.write(&hint_bytes).await?;

        trace!(target: "hint_writer", "Successfully wrote hint");

        // Read the hint acknowledgement from the host.
        let mut hint_ack = [0u8; 1];
        self.pipe_handle.read_exact(&mut hint_ack).await?;

        trace!(target: "hint_writer", "Received hint acknowledgement");

        Ok(())
    }

    /// Write a hint to the host. This will overwrite any existing hint in the pipe, and block until
    /// all data has been written.
    #[cfg(feature = "solo-client")]
    async fn write(&self, _: &str) -> Result<()> {
        Ok(())
    }
}

/// A [HintReader] is a router for hints sent by the [HintWriter] from the client program. It
/// provides a way for the host to prepare preimages for reading.
#[derive(Debug, Clone, Copy)]
pub struct HintReader {
    pipe_handle: PipeHandle,
}

impl HintReader {
    /// Create a new [HintReader] from a [PipeHandle].
    pub fn new(pipe_handle: PipeHandle) -> Self {
        Self { pipe_handle }
    }
}

#[async_trait]
impl HintReaderServer for HintReader {
    async fn next_hint<R>(&self, hint_router: &R) -> Result<()>
    where
        R: HintRouter + Send + Sync,
    {
        // Read the length of the raw hint payload.
        let mut len_buf = [0u8; 4];
        self.pipe_handle.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf);

        // Read the raw hint payload.
        let mut raw_payload = vec![0u8; len as usize];
        self.pipe_handle.read_exact(raw_payload.as_mut_slice()).await?;
        let payload = String::from_utf8(raw_payload)
            .map_err(|e| anyhow::anyhow!("Failed to decode hint payload: {e}"))?;

        trace!(target: "hint_reader", "Successfully read hint: \"{payload}\"");

        // Route the hint
        if let Err(e) = hint_router.route_hint(payload).await {
            // Write back on error to prevent blocking the client.
            self.pipe_handle.write(&[0x00]).await?;

            error!("Failed to route hint: {e}");
            anyhow::bail!("Failed to rout hint: {e}");
        }

        // Write back an acknowledgement to the client to unblock their process.
        self.pipe_handle.write(&[0x00]).await?;

        trace!(target: "hint_reader", "Successfully routed and acknowledged hint");

        Ok(())
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use alloc::{sync::Arc, vec::Vec};
    use kona_common::FileDescriptor;
    use std::{fs::File, os::fd::AsRawFd};
    use tempfile::tempfile;
    use tokio::sync::Mutex;

    /// Test struct containing the [HintReader] and [HintWriter]. The [File]s are stored in this
    /// struct so that they are not dropped until the end of the test.
    #[derive(Debug)]
    struct ClientAndHost {
        hint_writer: HintWriter,
        hint_reader: HintReader,
        _read_file: File,
        _write_file: File,
    }

    /// Helper for creating a new [HintReader] and [HintWriter] for testing. The file channel is
    /// over two temporary files.
    fn client_and_host() -> ClientAndHost {
        let (read_file, write_file) = (tempfile().unwrap(), tempfile().unwrap());
        let (read_fd, write_fd) = (
            FileDescriptor::Wildcard(read_file.as_raw_fd().try_into().unwrap()),
            FileDescriptor::Wildcard(write_file.as_raw_fd().try_into().unwrap()),
        );
        let client_handle = PipeHandle::new(read_fd, write_fd);
        let host_handle = PipeHandle::new(write_fd, read_fd);

        let hint_writer = HintWriter::new(client_handle);
        let hint_reader = HintReader::new(host_handle);

        ClientAndHost { hint_writer, hint_reader, _read_file: read_file, _write_file: write_file }
    }

    struct TestRouter {
        incoming_hints: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl HintRouter for TestRouter {
        async fn route_hint(&self, hint: String) -> Result<()> {
            self.incoming_hints.lock().await.push(hint);
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_hint_client_and_host() {
        const MOCK_DATA: &str = "test-hint 0xfacade";

        let sys = client_and_host();
        let (hint_writer, hint_reader) = (sys.hint_writer, sys.hint_reader);
        let incoming_hints = Arc::new(Mutex::new(Vec::new()));

        let client = tokio::task::spawn(async move { hint_writer.write(MOCK_DATA).await });
        let host = tokio::task::spawn({
            let incoming_hints_ref = Arc::clone(&incoming_hints);
            async move {
                let router = TestRouter { incoming_hints: incoming_hints_ref };
                hint_reader.next_hint(&router).await.unwrap();

                let mut hints = incoming_hints.lock().await;
                assert_eq!(hints.len(), 1);
                hints.remove(0)
            }
        });

        let (_, h) = tokio::join!(client, host);
        assert_eq!(h.unwrap(), MOCK_DATA);
    }
}
