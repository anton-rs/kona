use alloy_primitives::B256;
use std::str::FromStr;

/// Parse string slices into alloy_primitives bytes
///
/// # Arguments
/// * `s` - string slice
///
/// # Returns
/// * `Result<B256, String>` - Ok if successful, Err otherwise.
pub fn parse_b256(s: &str) -> Result<B256, String> {
    B256::from_str(s).map_err(|_| format!("Invalid B256 value: {}", s))
}