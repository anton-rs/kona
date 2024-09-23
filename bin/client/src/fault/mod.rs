//! Contains FPVM-specific constructs for the `kona-client` program.

use kona_common::FileDescriptor;
use kona_preimage::{HintWriter, OracleReader, PipeHandle};

mod handler;
pub(crate) use handler::fpvm_handle_register;

/// The global preimage oracle reader pipe.
static ORACLE_READER_PIPE: PipeHandle =
    PipeHandle::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite);

/// The global hint writer pipe.
static HINT_WRITER_PIPE: PipeHandle =
    PipeHandle::new(FileDescriptor::HintRead, FileDescriptor::HintWrite);

/// The global preimage oracle reader.
pub(crate) static ORACLE_READER: OracleReader = OracleReader::new(ORACLE_READER_PIPE);

/// The global hint writer.
pub(crate) static HINT_WRITER: HintWriter = HintWriter::new(HINT_WRITER_PIPE);
