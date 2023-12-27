//! TODO

use crate::PreimageKey;

/// A reader for the preimage oracle. This struct
pub struct OracleReader {
    key: Option<PreimageKey>,
    length: u64,
    cursor: u64,
}

// The only way to access an oracle reader is through this singleton. This is to ensure there cannot be more than one
// at a time which would have unpredictable results.
static mut ORACLE_READER: Option<OracleReader> = Some(OracleReader {
    key: None,
    length: 0,
    cursor: 0,
});

/// Get the global oracle reader
///
/// # Panics
/// This will panic if called more than once. This is to ensure there is only one oracle reader at once
/// as it encapsulates host global state.
pub fn oracle_reader() -> OracleReader {
    unsafe {
        let reader = core::ptr::replace(&mut ORACLE_READER, None);
        reader.expect("oracle_reader` has already been called. Can only call once per program")
    }
}
