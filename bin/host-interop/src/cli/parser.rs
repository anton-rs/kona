use alloy_primitives::{hex, Bytes, B256};
use std::error::Error;
use std::str::FromStr;

/// Parse string slices into alloy_primitives bytes
///
/// # Arguments
/// * `s` - string slice
///
/// # Returns
/// * `Result<B256, String>` - Ok if successful, Err otherwise.
pub(crate) fn parse_b256(s: &str) -> Result<B256, String> {
    B256::from_str(s).map_err(|_| format!("Invalid B256 value: {}", s))
}

pub(crate) fn parse_bytes(s: &str) -> Result<Bytes, String> {
    hex::decode(s).map_err(|e| format!("Invalid hex string: {}", e)).map(Bytes::from)
}

/// Parse a single key-value pair
pub(crate) fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s.find('=').ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}
