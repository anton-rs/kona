//! Contains utility functions and helpers for the host program.

use crate::types::NativePipeFiles;
use alloy_primitives::{hex, Bytes};
use alloy_provider::ReqwestProvider;
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use anyhow::{anyhow, Result};
use kona_client::HintType;
use kona_common::FileDescriptor;
use kona_preimage::PipeHandle;
use reqwest::Client;
use std::{fs::File, os::fd::AsRawFd};
use tokio::task::JoinHandle;

/// Parses a hint from a string.
///
/// Hints are of the format `<hint_type> <hint_data>`, where `<hint_type>` is a string that
/// represents the type of hint, and `<hint_data>` is the data associated with the hint
/// (bytes encoded as hex UTF-8).
pub(crate) fn parse_hint(s: &str) -> Result<(HintType, Bytes)> {
    let mut parts = s.split(' ').collect::<Vec<_>>();

    if parts.len() != 2 {
        anyhow::bail!("Invalid hint format: {}", s);
    }

    let hint_type = HintType::try_from(parts.remove(0))?;
    let hint_data = hex::decode(parts.remove(0)).map_err(|e| anyhow!(e))?.into();

    Ok((hint_type, hint_data))
}

/// Creates two temporary files that are connected by a pipe.
pub(crate) fn create_temp_files() -> Result<(File, File)> {
    let (read, write) = (
        tempfile::tempfile().map_err(|e| anyhow!(e))?,
        tempfile::tempfile().map_err(|e| anyhow!(e))?,
    );
    Ok((read, write))
}

/// Create a pair of pipes for the preimage oracle and hint reader. Also returns the files that are
/// used to create the pipes, which must be kept alive until the pipes are closed.
pub(crate) fn create_native_pipes() -> Result<(PipeHandle, PipeHandle, NativePipeFiles)> {
    let (po_reader, po_writer) = create_temp_files()?;
    let (hint_reader, hint_writer) = create_temp_files()?;
    let preimage_pipe = PipeHandle::new(
        FileDescriptor::Wildcard(
            po_reader.as_raw_fd().try_into().map_err(|e| anyhow!("Failed to get raw FD: {e}"))?,
        ),
        FileDescriptor::Wildcard(
            po_writer.as_raw_fd().try_into().map_err(|e| anyhow!("Failed to get raw FD: {e}"))?,
        ),
    );
    let hint_pipe = PipeHandle::new(
        FileDescriptor::Wildcard(
            hint_reader.as_raw_fd().try_into().map_err(|e| anyhow!("Failed to get raw FD: {e}"))?,
        ),
        FileDescriptor::Wildcard(
            hint_writer.as_raw_fd().try_into().map_err(|e| anyhow!("Failed to get raw FD: {e}"))?,
        ),
    );

    let files = NativePipeFiles {
        preimage_read: po_reader,
        preimage_writ: po_writer,
        hint_read: hint_reader,
        hint_writ: hint_writer,
    };

    Ok((preimage_pipe, hint_pipe, files))
}

/// Returns an HTTP provider for the given URL.
pub(crate) fn http_provider(url: &str) -> ReqwestProvider {
    let url = url.parse().unwrap();
    let http = Http::<Client>::new(url);
    ReqwestProvider::new(RpcClient::new(http, true))
}

/// Flattens the result of a [JoinHandle] into a single result.
pub(crate) async fn flatten_join_result<T, E>(
    handle: JoinHandle<Result<T, E>>,
) -> Result<T, anyhow::Error>
where
    E: std::fmt::Display,
{
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(anyhow!("{}", err)),
        Err(err) => anyhow::bail!(err),
    }
}
