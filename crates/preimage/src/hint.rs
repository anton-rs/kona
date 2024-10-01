use crate::{
    errors::{PreimageOracleError, PreimageOracleResult},
    traits::{HintRouter, HintWriterClient},
    HintReaderServer, PipeHandle,
};
use alloc::{boxed::Box, format, string::String, vec};
use async_trait::async_trait;
use tracing::{error, trace};

/// A [HintWriter] is a high-level interface to the hint pipe. It provides a way to write hints to
/// the host.
#[derive(Debug)]
pub struct HintWriter {
    pipe_handle: PipeHandle,
}

impl HintWriter {
    /// Create a new [HintWriter] from a [PipeHandle].
    pub const fn new(pipe_handle: PipeHandle) -> Self {
        Self { pipe_handle }
    }
}

#[async_trait]
impl HintWriterClient for HintWriter {
    /// Write a hint to the host. This will overwrite any existing hint in the pipe, and block until
    /// all data has been written.
    async fn write(&self, hint: &str) -> PreimageOracleResult<()> {
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
}

/// A [HintReader] is a router for hints sent by the [HintWriter] from the client program. It
/// provides a way for the host to prepare preimages for reading.
#[derive(Debug)]
pub struct HintReader {
    pipe_handle: PipeHandle,
}

impl HintReader {
    /// Create a new [HintReader] from a [PipeHandle].
    pub const fn new(pipe_handle: PipeHandle) -> Self {
        Self { pipe_handle }
    }
}

#[async_trait]
impl HintReaderServer for HintReader {
    async fn next_hint<R>(&self, hint_router: &R) -> PreimageOracleResult<()>
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
        let payload = match String::from_utf8(raw_payload) {
            Ok(p) => p,
            Err(e) => {
                // Write back on error to prevent blocking the client.
                self.pipe_handle.write(&[0x00]).await?;

                return Err(PreimageOracleError::Other(format!(
                    "Failed to decode hint payload: {e}"
                )));
            }
        };

        trace!(target: "hint_reader", "Successfully read hint: \"{payload}\"");

        // Route the hint
        if let Err(e) = hint_router.route_hint(payload).await {
            // Write back on error to prevent blocking the client.
            self.pipe_handle.write(&[0x00]).await?;

            error!("Failed to route hint: {e}");
            return Err(e);
        }

        // Write back an acknowledgement to the client to unblock their process.
        self.pipe_handle.write(&[0x00]).await?;

        trace!(target: "hint_reader", "Successfully routed and acknowledged hint");

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_utils::bidirectional_pipe;
    use alloc::{sync::Arc, vec::Vec};
    use kona_common::FileDescriptor;
    use std::os::fd::OwnedFd;
    use tokio::sync::Mutex;

    struct TestRouter {
        incoming_hints: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl HintRouter for TestRouter {
        async fn route_hint(&self, hint: String) -> PreimageOracleResult<()> {
            self.incoming_hints.lock().await.push(hint);
            Ok(())
        }
    }

    struct TestFailRouter;

    #[async_trait]
    impl HintRouter for TestFailRouter {
        async fn route_hint(&self, _hint: String) -> PreimageOracleResult<()> {
            Err(PreimageOracleError::KeyNotFound)
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_unblock_on_bad_utf8() {
        let mock_data = [0xf0, 0x90, 0x28, 0xbc];

        let hint_pipe = bidirectional_pipe().unwrap();

        let client = tokio::task::spawn(async move {
            let hint_writer = HintWriter::new(PipeHandle::new(
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.client.read)),
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.client.write)),
            ));

            #[allow(invalid_from_utf8_unchecked)]
            hint_writer.write(unsafe { alloc::str::from_utf8_unchecked(&mock_data) }).await
        });
        let host = tokio::task::spawn(async move {
            let router = TestRouter { incoming_hints: Default::default() };

            let hint_reader = HintReader::new(PipeHandle::new(
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.host.read)),
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.host.write)),
            ));
            hint_reader.next_hint(&router).await
        });

        let (c, h) = tokio::join!(client, host);
        c.unwrap().unwrap();
        assert!(h.unwrap().is_err_and(|e| {
            let PreimageOracleError::Other(e) = e else {
                return false;
            };
            e.contains("Failed to decode hint payload")
        }));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_unblock_on_fetch_failure() {
        const MOCK_DATA: &str = "test-hint 0xfacade";

        let hint_pipe = bidirectional_pipe().unwrap();

        let client = tokio::task::spawn(async move {
            let hint_writer = HintWriter::new(PipeHandle::new(
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.client.read)),
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.client.write)),
            ));

            hint_writer.write(MOCK_DATA).await
        });
        let host = tokio::task::spawn(async move {
            let hint_reader = HintReader::new(PipeHandle::new(
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.host.read)),
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.host.write)),
            ));
            hint_reader.next_hint(&TestFailRouter).await
        });

        let (c, h) = tokio::join!(client, host);
        c.unwrap().unwrap();
        assert!(h.unwrap().is_err_and(|e| matches!(e, PreimageOracleError::KeyNotFound)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_hint_client_and_host() {
        const MOCK_DATA: &str = "test-hint 0xfacade";

        let incoming_hints = Arc::new(Mutex::new(Vec::new()));
        let hint_pipe = bidirectional_pipe().unwrap();

        let client = tokio::task::spawn(async move {
            let hint_writer = HintWriter::new(PipeHandle::new(
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.client.read)),
                FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.client.write)),
            ));

            hint_writer.write(MOCK_DATA).await
        });
        let host = tokio::task::spawn({
            let incoming_hints_ref = Arc::clone(&incoming_hints);
            async move {
                let router = TestRouter { incoming_hints: incoming_hints_ref };

                let hint_reader = HintReader::new(PipeHandle::new(
                    FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.host.read)),
                    FileDescriptor::Wildcard(OwnedFd::from(hint_pipe.host.write)),
                ));
                hint_reader.next_hint(&router).await.unwrap();
            }
        });

        let _ = tokio::join!(client, host);
        let mut hints = incoming_hints.lock().await;

        assert_eq!(hints.len(), 1);
        let h = hints.remove(0);
        assert_eq!(h, MOCK_DATA);
    }
}
