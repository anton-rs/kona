//! This module contains the types used in the host program.

use std::fs::File;

/// Represents the files that are used to communicate with the native client.
#[derive(Debug)]
pub struct NativePipeFiles {
    /// The file that the preimage oracle reads from.
    pub preimage_read: File,
    /// The file that the preimage oracle writes to.
    pub preimage_writ: File,
    /// The file that the hint reader reads from.
    pub hint_read: File,
    /// The file that the hint reader writes to.
    pub hint_writ: File,
}
