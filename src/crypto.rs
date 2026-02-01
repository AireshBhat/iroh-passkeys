//! Cryptographic utilities module.
//!
//! This module provides helper functions for cryptographic operations
//! used throughout the wallet.

use subtle::ConstantTimeEq;

/// Compare two byte arrays in constant time.
///
/// This function prevents timing attacks by ensuring the comparison
/// takes the same amount of time regardless of how many bytes match.
///
/// # Arguments
///
/// * `a` - First byte array
/// * `b` - Second byte array
///
/// # Returns
///
/// Returns `true` if the arrays are equal, `false` otherwise.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

/// Validate that a slice has the expected length.
///
/// # Arguments
///
/// * `data` - The byte slice to validate
/// * `expected_len` - The expected length
///
/// # Returns
///
/// Returns `Ok(())` if the length matches, or an error message.
pub fn validate_length(data: &[u8], expected_len: usize) -> Result<(), String> {
    if data.len() != expected_len {
        Err(format!(
            "Invalid length: expected {}, got {}",
            expected_len,
            data.len()
        ))
    } else {
        Ok(())
    }
}

/// Securely clear a byte slice.
///
/// This function overwrites the slice with zeros.
/// Note: This is a best-effort operation; compiler optimizations
/// may prevent complete clearing.
pub fn secure_clear(data: &mut [u8]) {
    use zeroize::Zeroize;
    data.zeroize();
}

#[cfg(test)]
mod tests {}
