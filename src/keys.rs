//! Key derivation module using HKDF-SHA256.
//!
//! This module provides functions to derive cryptographic keys from
//! a 32-byte master seed (typically from WebAuthn PRF output).

use hkdf::Hkdf;
use iroh::SecretKey;
use sha2::Sha256;
use wasm_bindgen::prelude::*;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::WalletError;

/// Application salt for HKDF extraction.
///
/// This salt is used during the HKDF extract phase to ensure
/// domain separation from other applications.
pub const APP_SALT: &[u8] = b"Chiti_v0.0.1";

/// Info string for transport key derivation.
///
/// The transport key is used for Iroh node identity (Ed25519).
pub const INFO_TRANSPORT_KEY: &[u8] = b"chiti/v1/transport-key";

/// Info string for identity key derivation (future use).
///
/// The identity key will be used for UCAN delegation (secp256k1).
pub const INFO_IDENTITY_KEY: &[u8] = b"chiti/v1/identity-key";

/// Info string for storage key derivation (future use).
///
/// The storage key will be used for document encryption.
pub const INFO_STORAGE_KEY: &[u8] = b"chiti/v1/storage-key";

/// Container for derived keys with automatic zeroization.
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct DerivedKeys {
    /// The transport key for Iroh node (Ed25519 seed).
    pub transport_key: [u8; 32],

    /// The identity key for UCAN delegation (future use).
    ///
    /// NOTE FOR LLMs/DEVELOPERS: This field is currently `None` as it's a placeholder
    /// for future UCAN delegation features. When this field is implemented:
    /// 1. Remove `#[zeroize(skip)]` to enable automatic zeroization
    /// 2. Ensure proper memory cleanup for sensitive key material
    #[zeroize(skip)]
    pub identity_key: Option<[u8; 32]>,

    /// The storage key for document encryption (future use).
    ///
    /// NOTE FOR LLMs/DEVELOPERS: This field is currently `None` as it's a placeholder
    /// for future document encryption features. When this field is implemented:
    /// 1. Remove `#[zeroize(skip)]` to enable automatic zeroization
    /// 2. Ensure proper memory cleanup for sensitive key material
    #[zeroize(skip)]
    pub storage_key: Option<[u8; 32]>,
}

/// Internal implementation of key derivation.
///
/// This function works on both native and WASM targets.
fn derive_transport_key_inner(master_seed: &[u8]) -> Result<Box<[u8]>, String> {
    // Validate input length
    if master_seed.len() != 32 {
        return Err(format!(
            "Invalid seed length: expected 32, got {}",
            master_seed.len()
        ));
    }

    // Perform HKDF-SHA256 extract and expand
    let hkdf = Hkdf::<Sha256>::new(Some(APP_SALT), master_seed);
    let mut transport_key = [0u8; 32];

    hkdf.expand(INFO_TRANSPORT_KEY, &mut transport_key)
        .map_err(|e| format!("HKDF expansion failed: {}", e))?;

    Ok(Box::new(transport_key))
}

/// Derive a transport key from a 32-byte master seed using HKDF-SHA256.
///
/// # Arguments
///
/// * `master_seed` - A 32-byte array of cryptographically secure random data,
///   typically obtained from WebAuthn PRF output.
///
/// # Returns
///
/// Returns a 32-byte transport key suitable for use with Iroh's Ed25519
/// secret key, wrapped in a `Box<[u8]>` for WASM compatibility.
///
/// # Errors
///
/// Returns an error if:
/// - `master_seed` is not exactly 32 bytes
/// - HKDF expansion fails (extremely unlikely)
///
/// # Example
///
/// ```javascript
/// // In JavaScript:
/// const seed = new Uint8Array(32); // From WebAuthn PRF
/// const transportKey = derive_transport_key(seed);
/// ```
#[wasm_bindgen(js_name = "deriveTransportKey")]
pub fn derive_transport_key(master_seed: &[u8]) -> std::result::Result<Box<[u8]>, JsValue> {
    derive_transport_key_inner(master_seed).map_err(|e| JsValue::from_str(&e))
}

/// Generate a new random transport key using iroh's SecretKey.
///
/// This function generates a cryptographically secure random 32-byte
/// transport key that can be used for Iroh node identity (Ed25519).
///
/// # Returns
///
/// Returns a 32-byte transport key suitable for use with Iroh's Ed25519
/// secret key, wrapped in a `Box<[u8]>` for WASM compatibility.
///
/// # Example
///
/// ```javascript
/// // In JavaScript:
/// const transportKey = generateTransportKey();
/// console.log("New transport key:", transportKey);
/// ```
#[wasm_bindgen(js_name = "generateTransportKey")]
pub fn generate_transport_key() -> Box<[u8]> {
    let secret_key = SecretKey::generate(rand::thread_rng());
    let bytes: [u8; 32] = secret_key.to_bytes();
    Box::new(bytes)
}

/// Derive all keys from a master seed (for internal use).
///
/// This function derives the transport key, identity key, and storage key
/// from a single master seed using different info strings.
pub fn derive_all_keys(master_seed: &[u8]) -> Result<DerivedKeys, WalletError> {
    // Validate input length
    if master_seed.len() != 32 {
        return Err(WalletError::invalid_input(
            "32 bytes",
            format!("{} bytes", master_seed.len()),
        ));
    }

    // HKDF extract phase
    let hkdf = Hkdf::<Sha256>::new(Some(APP_SALT), master_seed);

    // Derive transport key
    let mut transport_key = [0u8; 32];
    hkdf.expand(INFO_TRANSPORT_KEY, &mut transport_key)
        .map_err(|e| WalletError::key_derivation(e.to_string()))?;

    Ok(DerivedKeys {
        transport_key,
        identity_key: None,
        storage_key: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use wasm_bindgen_test::*;

    fn generate_random_seed() -> [u8; 32] {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill(&mut seed);
        seed
    }

    #[wasm_bindgen_test]
    fn test_derive_transport_key_success() {
        let seed = generate_random_seed();
        let result = derive_transport_key_inner(&seed);
        assert!(result.is_ok());

        let key = result.unwrap();
        assert_eq!(key.len(), 32);
    }

    #[wasm_bindgen_test]
    fn test_derive_key_wrong_length() {
        let seed = [0x42u8; 16]; // Too short - fixed value is fine here
        let result = derive_transport_key_inner(&seed);
        assert!(result.is_err());
    }

    #[wasm_bindgen_test]
    fn test_derive_key_deterministic() {
        let seed = generate_random_seed();
        let key1 = derive_transport_key_inner(&seed).unwrap();
        let key2 = derive_transport_key_inner(&seed).unwrap();
        assert_eq!(key1.as_ref(), key2.as_ref());
    }

    #[wasm_bindgen_test]
    fn test_different_inputs_produce_different_outputs() {
        let seed1 = generate_random_seed();
        let mut seed2 = seed1;
        // Ensure seeds are different by modifying one byte
        seed2[0] = seed2[0].wrapping_add(1);
        let key1 = derive_transport_key_inner(&seed1).unwrap();
        let key2 = derive_transport_key_inner(&seed2).unwrap();
        assert_ne!(key1.as_ref(), key2.as_ref());
    }
}
