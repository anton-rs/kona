#[derive(Default, Clone, Debug)]
pub struct EigenDABlobData {
    /// The blob data
    pub(crate) version: Option<Bytes>,
    /// The calldata
    pub(crate) blob: Option<Bytes>,
}

impl EigenDABlobData {
    /// Decodes the blob into raw byte data.
    /// Returns a [BlobDecodingError] if the blob is invalid.
    pub(crate) fn decode(&self) -> Result<Bytes, BlobDecodingError> {
        // where we can implement zero bytes etc.
    }

}