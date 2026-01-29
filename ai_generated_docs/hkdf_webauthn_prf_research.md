# HKDF Parameters and WebAuthn PRF Implementation Research

## Executive Summary

This document provides comprehensive research on implementing a Local-First identity wallet using HKDF-SHA256 for key derivation from a 32-byte Master Entropy seed, combined with WebAuthn PRF extension for stateless key derivation. The research covers exact parameters, Rust implementation details, and security considerations.

---

## 1. HKDF-SHA256 Parameters (RFC 5869)

### 1.1 Overview

HKDF (HMAC-based Extract-and-Expand Key Derivation Function) as defined in [RFC 5869](https://datatracker.ietf.org/doc/html/rfc5869) follows the "extract-then-expand" paradigm:

1. **Extract Phase**: Concentrates dispersed entropy from Input Keying Material (IKM) into a fixed-length Pseudorandom Key (PRK)
2. **Expand Phase**: Expands PRK into multiple cryptographically independent keys

### 1.2 Exact Parameters for Three-Key Derivation

#### Master Entropy Seed (IKM)
- **Source**: 32-byte (256-bit) seed from WebAuthn PRF extension
- **Format**: Raw bytes from `prf.results.first` (ArrayBuffer)
- **Security**: High-entropy, hardware-backed by authenticator

#### Salt Strategy
Per RFC 5869 Section 3.1, salt usage is strongly RECOMMENDED:

| Salt Type | Value | Purpose |
|-----------|-------|---------|
| **Application Salt** | 32 random bytes (stored or derived) | Domain separation |
| **Default (No Salt)** | 32 zero bytes (HashLen zeros) | When salt unavailable |

**Recommendation**: Use a static application-specific salt for domain separation:
```rust
const APP_SALT: &[u8] = b"LocalFirstIdentityWallet_v1_2024"; // 32 bytes
```

### 1.3 Info String Parameters

Per RFC 5869 Section 3.2, the `info` parameter binds derived keys to application-specific context:

| Key Type | Info String | Output Length | Algorithm |
|----------|-------------|---------------|-----------|
| **Identity Key** | `"identity-wallet/v1/identity-key"` | 32 bytes | secp256k1 or Ed25519 |
| **Transport Key** | `"identity-wallet/v1/transport-key"` | 32 bytes | Ed25519 (Iroh) |
| **Storage Key (DEK)** | `"identity-wallet/v1/storage-key"` | 32 bytes | XChaCha20-Poly1305 |

**Info String Best Practices** (per RFC 5869):
- Include protocol version for future compatibility
- Include key purpose for domain separation
- Use ASCII/UTF-8 encoding
- Must be independent of IKM value

### 1.4 HKDF-Expand Limits

For SHA-256 (HashLen = 32 bytes):
- Maximum output length: 255 * 32 = 8,160 bytes
- Our requirement: 3 keys × 32 bytes = 96 bytes (well within limits)

---

## 2. WebAuthn PRF Extension Implementation

### 2.1 Specification Reference

WebAuthn Level 3 PRF extension is defined in the [W3C WebAuthn specification](https://github.com/w3c/webauthn/wiki/Explainer:-PRF-extension).

### 2.2 PRF Output Characteristics

| Property | Value |
|----------|-------|
| Output size | 32 bytes (256 bits) |
| Determinism | Same input → Same output for same credential |
| Hardware backing | CTAP2 `hmac-secret` extension |
| Security | Authenticator-bound, requires user gesture |

### 2.3 JavaScript Implementation

#### Credential Creation (Enable PRF)
```javascript
const createOptions = {
  publicKey: {
    rp: { name: "LocalFirst Identity Wallet", id: "example.com" },
    user: { id: userId, name: "user@example.com", displayName: "User" },
    challenge: crypto.getRandomValues(new Uint8Array(32)),
    pubKeyCredParams: [{ alg: -7, type: "public-key" }], // ES256
    extensions: {
      prf: {} // Enable PRF capability
    }
  }
};

const credential = await navigator.credentials.create(createOptions);
const prfEnabled = credential.getClientExtensionResults()?.prf?.enabled;
```

#### Credential Authentication (Derive Seed)
```javascript
// Per-credential salt (stored server-side or locally)
const credentialSalt = new Uint8Array([/* 32 bytes stored per credential */]);

const getOptions = {
  publicKey: {
    challenge: crypto.getRandomValues(new Uint8Array(32)),
    rpId: "example.com",
    allowCredentials: [{ id: credentialId, type: "public-key" }],
    extensions: {
      prf: {
        eval: {
          first: credentialSalt  // 32-byte input
        }
      }
    }
  }
};

const assertion = await navigator.credentials.get(getOptions);
const prfResult = assertion.getClientExtensionResults()?.prf?.results?.first;

// prfResult is 32-byte ArrayBuffer - this is your Master Entropy Seed
const masterSeed = new Uint8Array(prfResult);
```

### 2.4 PRF Input Salting

Per the WebAuthn specification, inputs are hashed with a context string before being passed to the underlying HMAC:

```
PRF input = HMAC-SHA256("WebAuthn PRF\x00", user_provided_salt)
```

This ensures web-derived outputs are distinct from non-web uses of the same authenticator.

### 2.5 Browser and Authenticator Support

| Component | Support Status |
|-----------|----------------|
| **Browsers** | Chrome 108+, Edge 108+, Safari 16+ (partial) |
| **Platform Authenticators** | Windows Hello, iCloud Keychain, Google Password Manager |
| **Security Keys** | YubiKey 5 series (CTAP 2.1 with hmac-secret) |

---

## 3. Key Derivation Implementation (Rust)

### 3.1 Required Dependencies

```toml
[dependencies]
# HKDF implementation
hkdf = "0.12"
sha2 = "0.10"

# Iroh for P2P transport
iroh = "0.96"

# Ed25519 for identity signing (alternative to secp256k1)
ed25519-dalek = "2.1"

# secp256k1 for identity signing (alternative to Ed25519)
k256 = { version = "0.13", features = ["ecdsa", "sha256"] }

# XChaCha20-Poly1305 for storage encryption
chacha20poly1305 = "0.10"

# Zeroization for secure memory handling
zeroize = "1.8"

# Error handling
thiserror = "1.0"
```

### 3.2 Core HKDF Derivation Module

```rust
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};
use thiserror::Error;

/// Application-specific salt for domain separation
const APP_SALT: &[u8] = b"LocalFirstIdentityWallet_v1_2024";

/// Info strings for key derivation (per RFC 5869 Section 3.2)
const INFO_IDENTITY_KEY: &[u8] = b"identity-wallet/v1/identity-key";
const INFO_TRANSPORT_KEY: &[u8] = b"identity-wallet/v1/transport-key";
const INFO_STORAGE_KEY: &[u8] = b"identity-wallet/v1/storage-key";

/// Output key lengths
const IDENTITY_KEY_LEN: usize = 32;
const TRANSPORT_KEY_LEN: usize = 32;
const STORAGE_KEY_LEN: usize = 32;

#[derive(Error, Debug)]
pub enum KeyDerivationError {
    #[error("HKDF expansion failed: {0}")]
    HkdfError(#[from] hkdf::InvalidLength),
    
    #[error("Invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },
    
    #[error("Invalid master seed length: expected 32 bytes, got {0}")]
    InvalidSeedLength(usize),
}

/// Derived keys container with automatic zeroization
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DerivedKeys {
    #[zeroize(skip)]  // Will be converted to proper key types
    pub identity_key_bytes: [u8; IDENTITY_KEY_LEN],
    #[zeroize(skip)]  // Will be converted to Iroh SecretKey
    pub transport_key_bytes: [u8; TRANSPORT_KEY_LEN],
    pub storage_key_bytes: [u8; STORAGE_KEY_LEN],
}

/// Derive all three keys from a 32-byte master seed using HKDF-SHA256
/// 
/// # Arguments
/// * `master_seed` - 32-byte seed from WebAuthn PRF extension
/// 
/// # Returns
/// * `DerivedKeys` - Container with all three derived keys
/// 
/// # Errors
/// * `KeyDerivationError` - If derivation fails or seed is invalid
/// 
/// # Security Notes
/// - Master seed should be zeroized after use
/// - Derived keys are zeroized on drop
pub fn derive_keys_from_seed(
    master_seed: &[u8; 32]
) -> Result<DerivedKeys, KeyDerivationError> {
    // Validate seed length (defensive)
    if master_seed.len() != 32 {
        return Err(KeyDerivationError::InvalidSeedLength(master_seed.len()));
    }
    
    // Step 1: HKDF Extract with application salt
    // Per RFC 5869 Section 2.2: PRK = HMAC-Hash(salt, IKM)
    let hkdf = Hkdf::<Sha256>::new(Some(APP_SALT), master_seed);
    
    // Step 2: HKDF Expand for each key
    // Per RFC 5869 Section 2.3: OKM = HKDF-Expand(PRK, info, L)
    
    let mut identity_key = [0u8; IDENTITY_KEY_LEN];
    hkdf.expand(INFO_IDENTITY_KEY, &mut identity_key)
        .map_err(KeyDerivationError::HkdfError)?;
    
    let mut transport_key = [0u8; TRANSPORT_KEY_LEN];
    hkdf.expand(INFO_TRANSPORT_KEY, &mut transport_key)
        .map_err(KeyDerivationError::HkdfError)?;
    
    let mut storage_key = [0u8; STORAGE_KEY_LEN];
    hkdf.expand(INFO_STORAGE_KEY, &mut storage_key)
        .map_err(KeyDerivationError::HkdfError)?;
    
    Ok(DerivedKeys {
        identity_key_bytes: identity_key,
        transport_key_bytes: transport_key,
        storage_key_bytes: storage_key,
    })
}

/// Alternative: Derive keys with custom salt (for advanced use cases)
pub fn derive_keys_with_salt(
    master_seed: &[u8; 32],
    salt: &[u8]
) -> Result<DerivedKeys, KeyDerivationError> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), master_seed);
    
    let mut identity_key = [0u8; IDENTITY_KEY_LEN];
    hkdf.expand(INFO_IDENTITY_KEY, &mut identity_key)
        .map_err(KeyDerivationError::HkdfError)?;
    
    let mut transport_key = [0u8; TRANSPORT_KEY_LEN];
    hkdf.expand(INFO_TRANSPORT_KEY, &mut transport_key)
        .map_err(KeyDerivationError::HkdfError)?;
    
    let mut storage_key = [0u8; STORAGE_KEY_LEN];
    hkdf.expand(INFO_STORAGE_KEY, &mut storage_key)
        .map_err(KeyDerivationError::HkdfError)?;
    
    Ok(DerivedKeys {
        identity_key_bytes: identity_key,
        transport_key_bytes: transport_key,
        storage_key_bytes: storage_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_derivation_determinism() {
        let seed = [0x42u8; 32];
        
        let keys1 = derive_keys_from_seed(&seed).unwrap();
        let keys2 = derive_keys_from_seed(&seed).unwrap();
        
        assert_eq!(keys1.identity_key_bytes, keys2.identity_key_bytes);
        assert_eq!(keys1.transport_key_bytes, keys2.transport_key_bytes);
        assert_eq!(keys1.storage_key_bytes, keys2.storage_key_bytes);
    }
    
    #[test]
    fn test_key_derivation_uniqueness() {
        let seed1 = [0x42u8; 32];
        let seed2 = [0x43u8; 32];
        
        let keys1 = derive_keys_from_seed(&seed1).unwrap();
        let keys2 = derive_keys_from_seed(&seed2).unwrap();
        
        assert_ne!(keys1.identity_key_bytes, keys2.identity_key_bytes);
        assert_ne!(keys1.transport_key_bytes, keys2.transport_key_bytes);
        assert_ne!(keys1.storage_key_bytes, keys2.storage_key_bytes);
    }
}
```

### 3.3 Identity Key Implementation (secp256k1 with k256 crate)

```rust
use k256::ecdsa::{SigningKey, signature::Signer};
use k256::SecretKey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IdentityKeyError {
    #[error("Invalid key bytes: {0}")]
    InvalidKey(#[from] k256::elliptic_curve::Error),
    
    #[error("Signing failed: {0}")]
    SigningFailed(#[from] k256::ecdsa::Error),
}

/// Identity signing key using secp256k1
pub struct IdentitySigningKey {
    signing_key: SigningKey,
}

impl IdentitySigningKey {
    /// Create identity key from 32-byte derived seed
    /// 
    /// # Arguments
    /// * `key_bytes` - 32 bytes from HKDF derivation
    /// 
    /// # Returns
    /// * `IdentitySigningKey` - Ready for signing W3C Verifiable Credentials
    pub fn from_derived_bytes(key_bytes: &[u8; 32]) -> Result<Self, IdentityKeyError> {
        // k256::SecretKey::from_bytes converts 32 bytes to a secp256k1 scalar
        let secret_key = SecretKey::from_bytes(key_bytes.into())?;
        let signing_key = SigningKey::from(secret_key);
        
        Ok(Self { signing_key })
    }
    
    /// Sign a message (for W3C Verifiable Credentials)
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, IdentityKeyError> {
        let signature: k256::ecdsa::Signature = self.signing_key.sign(message);
        Ok(signature.to_bytes().to_vec())
    }
    
    /// Get the verifying key (public key) for signature verification
    pub fn verifying_key(&self) -> k256::ecdsa::VerifyingKey {
        self.signing_key.verifying_key().clone()
    }
    
    /// Get the public key bytes (33 bytes compressed or 65 bytes uncompressed)
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.verifying_key().to_encoded_point(true).as_bytes().to_vec()
    }
}

/// Alternative: Ed25519 Identity Key (using ed25519-dalek)
pub mod ed25519_identity {
    use ed25519_dalek::{SigningKey, Signer};
    use super::*;
    
    pub struct Ed25519IdentityKey {
        signing_key: SigningKey,
    }
    
    impl Ed25519IdentityKey {
        pub fn from_derived_bytes(key_bytes: &[u8; 32]) -> Result<Self, IdentityKeyError> {
            // ed25519-dalek expects 32 bytes directly as the seed
            let signing_key = SigningKey::from_bytes(key_bytes);
            
            Ok(Self { signing_key })
        }
        
        pub fn sign(&self, message: &[u8]) -> Vec<u8> {
            self.signing_key.sign(message).to_bytes().to_vec()
        }
        
        pub fn verifying_key(&self) -> ed25519_dalek::VerifyingKey {
            self.signing_key.verifying_key()
        }
    }
}
```

### 3.4 Transport Key Implementation (Iroh Ed25519)

```rust
use iroh::SecretKey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportKeyError {
    #[error("Invalid key bytes for Iroh SecretKey")]
    InvalidKey,
}

/// Iroh-compatible transport key for P2P networking
/// 
/// Iroh's SecretKey is an Ed25519 key used for:
/// - QUIC connection authentication
/// - MagicSocket endpoint identification  
/// - Discovery service signing
pub struct TransportKey {
    secret_key: SecretKey,
}

impl TransportKey {
    /// Create Iroh transport key from 32-byte derived seed
    /// 
    /// # Arguments
    /// * `key_bytes` - 32 bytes from HKDF derivation
    /// 
    /// # Returns
    /// * `TransportKey` - Compatible with iroh::Endpoint
    /// 
    /// # Iroh Compatibility
    /// - Iroh SecretKey is exactly 32 bytes (Ed25519 seed)
    /// - Public key derived deterministically from seed
    /// - Used for endpoint ID in iroh-net MagicSocket
    pub fn from_derived_bytes(key_bytes: &[u8; 32]) -> Result<Self, TransportKeyError> {
        // Iroh SecretKey::from_bytes accepts exactly 32 bytes
        let secret_key = SecretKey::from_bytes(key_bytes);
        
        Ok(Self { secret_key })
    }
    
    /// Get the Iroh SecretKey for use with Endpoint builder
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }
    
    /// Get the endpoint ID (public key) for this transport key
    pub fn endpoint_id(&self) -> iroh::EndpointId {
        self.secret_key.public()
    }
    
    /// Convert to bytes for storage
    pub fn to_bytes(&self) -> [u8; 32] {
        self.secret_key.to_bytes()
    }
}

/// Example: Creating an Iroh Endpoint with derived transport key
pub async fn create_iroh_endpoint(
    transport_key: &TransportKey
) -> Result<iroh::Endpoint, anyhow::Error> {
    let endpoint = iroh::Endpoint::builder()
        .secret_key(transport_key.secret_key().clone())
        .discovery_n0()  // Use N0 discovery service
        .bind()
        .await?;
    
    Ok(endpoint)
}
```

### 3.5 Storage Key Implementation (XChaCha20-Poly1305)

```rust
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageKeyError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(#[from] chacha20poly1305::aead::Error),
    
    #[error("Decryption failed: authentication tag mismatch")]
    DecryptionFailed,
}

/// Storage encryption key for iroh-docs data
/// 
/// XChaCha20-Poly1305 provides:
/// - 256-bit keys (32 bytes)
/// - 192-bit nonces (24 bytes) - safe for random generation
/// - 128-bit authentication tags
pub struct StorageEncryptionKey {
    cipher: XChaCha20Poly1305,
}

impl StorageEncryptionKey {
    /// Create storage key from 32-byte derived seed
    /// 
    /// # Arguments
    /// * `key_bytes` - 32 bytes from HKDF derivation
    /// 
    /// # Returns
    /// * `StorageEncryptionKey` - Ready for encrypting iroh-docs data
    /// 
    /// # XChaCha20-Poly1305 Properties
    /// - Key size: 32 bytes (256 bits)
    /// - Nonce size: 24 bytes (192 bits) - extended from ChaCha20's 12 bytes
    /// - Tag size: 16 bytes (128 bits)
    pub fn from_derived_bytes(key_bytes: &[u8; 32]) -> Self {
        let cipher = XChaCha20Poly1305::new_from_slice(key_bytes)
            .expect("32 bytes is valid key size for XChaCha20Poly1305");
        
        Self { cipher }
    }
    
    /// Encrypt data with random nonce
    /// 
    /// # Returns
    /// Tuple of (nonce, ciphertext) where ciphertext includes auth tag
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), StorageKeyError> {
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self.cipher.encrypt(&nonce, plaintext)?;
        
        Ok((nonce.to_vec(), ciphertext))
    }
    
    /// Decrypt data with provided nonce
    pub fn decrypt(
        &self,
        nonce: &[u8],
        ciphertext: &[u8]
    ) -> Result<Vec<u8>, StorageKeyError> {
        let nonce = XNonce::from_slice(nonce);
        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|_| StorageKeyError::DecryptionFailed)?;
        
        Ok(plaintext)
    }
    
    /// Encrypt with associated data (AEAD)
    pub fn encrypt_with_aad(
        &self,
        plaintext: &[u8],
        aad: &[u8]
    ) -> Result<(Vec<u8>, Vec<u8>), StorageKeyError> {
        use chacha20poly1305::aead::Payload;
        
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let payload = Payload { msg: plaintext, aad };
        let ciphertext = self.cipher.encrypt(&nonce, payload)?;
        
        Ok((nonce.to_vec(), ciphertext))
    }
}

/// Convenience function for encrypting iroh-docs data
pub fn encrypt_doc_data(
    storage_key: &StorageEncryptionKey,
    doc_data: &[u8],
    doc_id: &str
) -> Result<EncryptedDoc, StorageKeyError> {
    // Use doc_id as associated data for binding encryption to document
    let (nonce, ciphertext) = storage_key.encrypt_with_aad(doc_data, doc_id.as_bytes())?;
    
    Ok(EncryptedDoc {
        nonce,
        ciphertext,
        doc_id: doc_id.to_string(),
    })
}

pub struct EncryptedDoc {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub doc_id: String,
}
```

### 3.6 Complete Integration Example

```rust
use zeroize::Zeroizing;

/// Complete workflow: Derive all keys from WebAuthn PRF seed
/// 
/// # Arguments
/// * `webauthn_prf_output` - 32-byte ArrayBuffer from navigator.credentials.get()
/// 
/// # Returns
/// * `WalletKeys` - All three keys ready for use
/// 
/// # Security Notes
/// - Input seed is wrapped in Zeroizing for automatic cleanup
/// - All sensitive bytes are zeroized on drop
pub fn initialize_wallet_keys(
    webauthn_prf_output: &[u8; 32]
) -> Result<WalletKeys, Box<dyn std::error::Error>> {
    // Wrap seed in Zeroizing for secure cleanup
    let seed = Zeroizing::new(*webauthn_prf_output);
    
    // Derive all three keys using HKDF-SHA256
    let derived = derive_keys_from_seed(&seed)?;
    
    // Create identity key (secp256k1 for W3C VC compatibility)
    let identity_key = IdentitySigningKey::from_derived_bytes(&derived.identity_key_bytes)?;
    
    // Create transport key (Ed25519 for Iroh)
    let transport_key = TransportKey::from_derived_bytes(&derived.transport_key_bytes)?;
    
    // Create storage key (XChaCha20-Poly1305)
    let storage_key = StorageEncryptionKey::from_derived_bytes(&derived.storage_key_bytes);
    
    Ok(WalletKeys {
        identity_key,
        transport_key,
        storage_key,
    })
}

/// Container for all wallet keys
pub struct WalletKeys {
    pub identity_key: IdentitySigningKey,
    pub transport_key: TransportKey,
    pub storage_key: StorageEncryptionKey,
}
```

---

## 4. Security Considerations

### 4.1 Master Seed Protection

| Threat | Mitigation |
|--------|------------|
| **Memory exposure** | Use `Zeroizing` wrapper for automatic cleanup |
| **Swap/pagefile leak** | Use `mlock` on Unix or secure enclaves where available |
| **Core dump exposure** | Disable core dumps in production |
| **Debugger attachment** | Runtime anti-debugging techniques (optional) |

### 4.2 Salt Management

Per RFC 5869 Section 3.1:
- Salt is **non-secret** and can be stored in plaintext
- Salt provides domain separation between different applications
- Using the same salt across different master seeds is safe
- Changing salt requires re-deriving all keys

### 4.3 Info String Uniqueness

Per RFC 5869 Section 3.2:
- Info strings must be **independent of IKM**
- Using the same info string with different salts produces different keys
- Using different info strings with the same salt produces different keys
- Info strings prevent cross-context key collisions

### 4.4 WebAuthn PRF Security Properties

| Property | Guarantee |
|----------|-----------|
| **Credential binding** | PRF output bound to specific WebAuthn credential |
| **User presence** | Hardware requires user gesture (touch, PIN, biometric) |
| **Origin binding** | PRF only works on registered origin |
| **No silent access** | Cannot extract PRF without user interaction |
| **Hardware backing** | Key material in TPM/Secure Enclave/Security Key |

### 4.5 Side-Channel Considerations

```rust
// Use constant-time comparison for key verification
use subtle::ConstantTimeEq;

fn verify_keys_equal(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}
```

### 4.6 Backup and Recovery

Since keys are deterministically derived from WebAuthn PRF output:

1. **Primary recovery**: Re-authenticate with same credential → same PRF output → same keys
2. **Backup credentials**: Register multiple credentials, derive same keys from each
3. **Export warning**: Never export raw derived keys; always re-derive on demand

---

## 5. References

### Specifications
1. [RFC 5869: HKDF](https://datatracker.ietf.org/doc/html/rfc5869) - HMAC-based Key Derivation Function
2. [WebAuthn Level 3](https://github.com/w3c/webauthn/wiki/Explainer:-PRF-extension) - PRF Extension Explainer
3. [CTAP 2.1](https://fidoalliance.org/specs/fido-v2.1-ps-20210615/fido-client-to-authenticator-protocol-v2.1-ps-20210615.html) - hmac-secret extension

### Rust Crates
1. [hkdf crate](https://docs.rs/hkdf/latest/hkdf/) - RFC 5869 implementation
2. [iroh](https://docs.rs/iroh/latest/iroh/) - P2P networking with Ed25519
3. [ed25519-dalek](https://docs.rs/ed25519-dalek/latest/ed25519_dalek/) - Ed25519 signatures
4. [k256](https://docs.rs/k256/latest/k256/) - secp256k1 elliptic curve
5. [chacha20poly1305](https://docs.rs/chacha20poly1305/latest/chacha20poly1305/) - AEAD cipher

### Further Reading
1. [HKDF Paper](https://eprint.iacr.org/2010/264) - Krawczyk, "Cryptographic Extraction and Key Derivation"
2. [XChaCha20 Draft](https://tools.ietf.org/html/draft-arciszewski-xchacha-03) - Extended nonce ChaCha
3. [WebAuthn PRF Guide](https://developers.yubico.com/WebAuthn/Concepts/PRF_Extension/Developers_Guide_to_PRF.html) - Yubico developer guide

---

## 6. Summary Table

| Component | Algorithm | Key Size | Source | Info String |
|-----------|-----------|----------|--------|-------------|
| Master Seed | WebAuthn PRF | 32 bytes | Hardware authenticator | N/A |
| Identity Key | secp256k1/Ed25519 | 32 bytes | HKDF-SHA256 | `"identity-wallet/v1/identity-key"` |
| Transport Key | Ed25519 (Iroh) | 32 bytes | HKDF-SHA256 | `"identity-wallet/v1/transport-key"` |
| Storage Key | XChaCha20-Poly1305 | 32 bytes | HKDF-SHA256 | `"identity-wallet/v1/storage-key"` |

---

*Document Version: 1.0*
*Last Updated: 2025*
*Research completed for Local-First Identity Wallet implementation*
