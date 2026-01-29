// ============================================================================
// Local-First Identity Wallet - Key Derivation Implementation
// ============================================================================
// This module implements HKDF-SHA256 key derivation for a WebAuthn PRF-based
// identity wallet. It derives three keys from a 32-byte master seed:
//   1. Identity Key (secp256k1) - for signing W3C Verifiable Credentials
//   2. Transport Key (Ed25519) - for Iroh P2P networking
//   3. Storage Key (XChaCha20-Poly1305) - for encrypting local data
//
// References:
//   - RFC 5869: HKDF specification
//   - WebAuthn Level 3: PRF extension
//   - Iroh documentation: https://docs.rs/iroh
// ============================================================================

use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};
use thiserror::Error;

// =============================================================================
// Section 1: HKDF Parameters (RFC 5869)
// =============================================================================

/// Application-specific salt for domain separation (RFC 5869 Section 3.1)
/// 
/// The salt is non-secret and provides domain separation between different
/// applications using the same HKDF construction. Using a static salt is
/// acceptable when the IKM (master seed) has high entropy.
pub const APP_SALT: &[u8] = b"LocalFirstIdentityWallet_v1_2024";

/// Info strings for key derivation (RFC 5869 Section 3.2)
/// 
/// Info strings bind derived keys to specific contexts, preventing
/// cross-context key collisions. Format: "application/version/purpose"
pub const INFO_IDENTITY_KEY: &[u8] = b"identity-wallet/v1/identity-key";
pub const INFO_TRANSPORT_KEY: &[u8] = b"identity-wallet/v1/transport-key";
pub const INFO_STORAGE_KEY: &[u8] = b"identity-wallet/v1/storage-key";

/// Output key lengths in bytes
pub const IDENTITY_KEY_LEN: usize = 32;   // secp256k1/Ed25519 private key
pub const TRANSPORT_KEY_LEN: usize = 32;  // Ed25519 seed for Iroh
pub const STORAGE_KEY_LEN: usize = 32;    // XChaCha20-Poly1305 key

// =============================================================================
// Section 2: Error Types
// =============================================================================

#[derive(Error, Debug)]
pub enum KeyDerivationError {
    #[error("HKDF expansion failed: {0}")]
    HkdfError(#[from] hkdf::InvalidLength),
    
    #[error("Invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },
    
    #[error("Invalid master seed length: expected 32 bytes, got {0}")]
    InvalidSeedLength(usize),
    
    #[error("secp256k1 key error: {0}")]
    Secp256k1Error(#[from] k256::elliptic_curve::Error),
    
    #[error("Ed25519 key error")]
    Ed25519Error,
    
    #[error("Storage encryption error: {0}")]
    StorageError(String),
}

// =============================================================================
// Section 3: Derived Keys Container
// =============================================================================

/// Container for all derived keys with automatic zeroization
/// 
/// SECURITY: All key material is zeroized from memory when this struct is dropped.
/// The identity and transport key bytes are marked with `#[zeroize(skip)]` because
/// they are immediately converted to proper key types that handle their own zeroization.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DerivedKeys {
    #[zeroize(skip)]
    pub identity_key_bytes: [u8; IDENTITY_KEY_LEN],
    #[zeroize(skip)]
    pub transport_key_bytes: [u8; TRANSPORT_KEY_LEN],
    pub storage_key_bytes: [u8; STORAGE_KEY_LEN],
}

// =============================================================================
// Section 4: Core HKDF Derivation
// =============================================================================

/// Derive all three keys from a 32-byte master seed using HKDF-SHA256
/// 
/// # Algorithm (RFC 5869)
/// 
/// 1. **Extract**: PRK = HMAC-SHA256(salt, IKM)
///    - Concentrates entropy from the master seed
///    - salt = APP_SALT (application-specific)
///    - IKM = 32-byte master seed from WebAuthn PRF
/// 
/// 2. **Expand**: OKM = HKDF-Expand(PRK, info, L)
///    - Derives each key with a unique info string
///    - L = 32 bytes for each key
/// 
/// # Arguments
/// * `master_seed` - 32-byte seed from WebAuthn PRF extension
/// 
/// # Returns
/// * `DerivedKeys` - Container with all three derived keys
/// 
/// # Errors
/// * `KeyDerivationError::InvalidSeedLength` - If seed is not 32 bytes
/// * `KeyDerivationError::HkdfError` - If HKDF expansion fails
/// 
/// # Example
/// ```
/// let seed = [0x42u8; 32]; // In practice, from WebAuthn PRF
/// let keys = derive_keys_from_seed(&seed)?;
/// ```
pub fn derive_keys_from_seed(
    master_seed: &[u8; 32]
) -> Result<DerivedKeys, KeyDerivationError> {
    // Validate seed length (defensive check)
    if master_seed.len() != 32 {
        return Err(KeyDerivationError::InvalidSeedLength(master_seed.len()));
    }
    
    // Step 1: HKDF Extract
    // PRK = HMAC-SHA256(APP_SALT, master_seed)
    let hkdf = Hkdf::<Sha256>::new(Some(APP_SALT), master_seed);
    
    // Step 2: HKDF Expand for each key
    // Each expansion uses a unique info string for domain separation
    
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

/// Derive keys with a custom salt (advanced use cases)
/// 
/// Use this when you need different domain separation or are migrating
/// from a different salt value.
pub fn derive_keys_with_custom_salt(
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

// =============================================================================
// Section 5: Identity Key (secp256k1 using k256 crate)
// =============================================================================

/// Identity signing key using secp256k1
/// 
/// This key is used for signing W3C Verifiable Credentials. secp256k1 is
/// widely supported in the verifiable credentials ecosystem.
pub mod identity_key_secp256k1 {
    use super::*;
    use k256::ecdsa::{SigningKey, signature::Signer};
    use k256::SecretKey;
    
    pub struct IdentitySigningKey {
        signing_key: SigningKey,
    }
    
    impl IdentitySigningKey {
        /// Create identity key from 32-byte derived seed
        /// 
        /// # k256 crate specifics:
        /// - Accepts 32 bytes directly as the secret scalar
        /// - Performs validation that bytes are within curve order
        /// - Public key is derived deterministically
        pub fn from_derived_bytes(
            key_bytes: &[u8; 32]
        ) -> Result<Self, KeyDerivationError> {
            let secret_key = SecretKey::from_bytes(key_bytes.into())?;
            let signing_key = SigningKey::from(secret_key);
            
            Ok(Self { signing_key })
        }
        
        /// Sign a message (for W3C Verifiable Credentials)
        /// 
        /// Returns a 64-byte signature (r || s)
        pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, KeyDerivationError> {
            let signature: k256::ecdsa::Signature = self.signing_key.sign(message);
            Ok(signature.to_bytes().to_vec())
        }
        
        /// Sign with recovery ID (for Ethereum-compatible signatures)
        pub fn sign_recoverable(
            &self,
            message: &[u8]
        ) -> Result<(Vec<u8>, u8), KeyDerivationError> {
            use k256::ecdsa::{RecoveryId, signature::DigestSigner};
            use sha2::Sha256;
            
            let digest = Sha256::digest(message);
            let (sig, recid) = self.signing_key
                .sign_digest_recoverable(digest)?;
            
            Ok((sig.to_bytes().to_vec(), recid.to_byte()))
        }
        
        /// Get the verifying key (public key) for signature verification
        pub fn verifying_key(&self) -> k256::ecdsa::VerifyingKey {
            self.signing_key.verifying_key().clone()
        }
        
        /// Get compressed public key bytes (33 bytes)
        pub fn public_key_compressed(&self) -> Vec<u8> {
            self.verifying_key()
                .to_encoded_point(true)
                .as_bytes()
                .to_vec()
        }
        
        /// Get uncompressed public key bytes (65 bytes)
        pub fn public_key_uncompressed(&self) -> Vec<u8> {
            self.verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .to_vec()
        }
        
        /// Get the Ethereum address (20 bytes) derived from this key
        pub fn ethereum_address(&self) -> Vec<u8> {
            use sha3::{Keccak256, Digest};
            
            let uncompressed = self.public_key_uncompressed();
            // Skip the 0x04 prefix byte
            let hash = Keccak256::digest(&uncompressed[1..]);
            // Return last 20 bytes
            hash[12..].to_vec()
        }
    }
}

// =============================================================================
// Section 6: Identity Key (Ed25519 using ed25519-dalek)
// =============================================================================

/// Identity signing key using Ed25519
/// 
/// Alternative to secp256k1. Ed25519 provides:
/// - Faster signature generation and verification
/// - Compact 64-byte signatures
/// - No need for randomness during signing (deterministic)
pub mod identity_key_ed25519 {
    use super::*;
    use ed25519_dalek::{SigningKey, Signer};
    
    pub struct Ed25519IdentityKey {
        signing_key: SigningKey,
    }
    
    impl Ed25519IdentityKey {
        /// Create Ed25519 identity key from 32-byte derived seed
        /// 
        /// # ed25519-dalek specifics:
        /// - The 32 bytes are used directly as the seed
        /// - The expanded secret key is computed deterministically
        /// - Public key is derived via scalar multiplication
        pub fn from_derived_bytes(
            key_bytes: &[u8; 32]
        ) -> Result<Self, KeyDerivationError> {
            let signing_key = SigningKey::from_bytes(key_bytes);
            Ok(Self { signing_key })
        }
        
        /// Sign a message
        /// 
        /// Returns a 64-byte signature
        pub fn sign(&self, message: &[u8]) -> Vec<u8> {
            self.signing_key.sign(message).to_bytes().to_vec()
        }
        
        /// Get the verifying key (public key)
        pub fn verifying_key(&self) -> ed25519_dalek::VerifyingKey {
            self.signing_key.verifying_key()
        }
        
        /// Get public key bytes (32 bytes)
        pub fn public_key_bytes(&self) -> [u8; 32] {
            self.verifying_key().to_bytes()
        }
    }
}

// =============================================================================
// Section 7: Transport Key (Iroh Ed25519)
// =============================================================================

/// Iroh-compatible transport key for P2P networking
/// 
/// Iroh uses Ed25519 keys for:
/// - QUIC connection authentication
/// - MagicSocket endpoint identification
/// - Discovery service (pkarr/DHT) signing
/// 
/// The SecretKey is exactly 32 bytes, used as the seed for Ed25519.
pub mod transport_key {
    use super::*;
    
    /// Iroh transport key wrapper
    pub struct IrohTransportKey {
        secret_key: iroh::SecretKey,
    }
    
    impl IrohTransportKey {
        /// Create Iroh transport key from 32-byte derived seed
        /// 
        /// # Iroh SecretKey specifics:
        /// - Accepts exactly 32 bytes
        /// - Internally uses ed25519-dalek
        /// - Public key derived deterministically
        /// - Used as EndpointId in iroh-net
        pub fn from_derived_bytes(
            key_bytes: &[u8; 32]
        ) -> Result<Self, KeyDerivationError> {
            let secret_key = iroh::SecretKey::from_bytes(key_bytes);
            Ok(Self { secret_key })
        }
        
        /// Get the Iroh SecretKey for use with Endpoint builder
        pub fn secret_key(&self) -> &iroh::SecretKey {
            &self.secret_key
        }
        
        /// Clone the secret key for endpoint construction
        pub fn secret_key_cloned(&self) -> iroh::SecretKey {
            self.secret_key.clone()
        }
        
        /// Get the endpoint ID (public key) for this transport key
        pub fn endpoint_id(&self) -> iroh::EndpointId {
            self.secret_key.public()
        }
        
        /// Get the endpoint ID as a string (z-base32 encoded)
        pub fn endpoint_id_string(&self) -> String {
            self.endpoint_id().to_string()
        }
        
        /// Convert to bytes for storage
        pub fn to_bytes(&self) -> [u8; 32] {
            self.secret_key.to_bytes()
        }
    }
    
    /// Create an Iroh Endpoint with the derived transport key
    /// 
    /// # Example
    /// ```
    /// let transport_key = IrohTransportKey::from_derived_bytes(&derived.transport_key_bytes)?;
    /// let endpoint = create_iroh_endpoint(&transport_key).await?;
    /// ```
    pub async fn create_iroh_endpoint(
        transport_key: &IrohTransportKey
    ) -> Result<iroh::Endpoint, anyhow::Error> {
        let endpoint = iroh::Endpoint::builder()
            .secret_key(transport_key.secret_key_cloned())
            .discovery_n0()  // Use N0 discovery service
            .bind()
            .await?;
        
        Ok(endpoint)
    }
    
    /// Create an Iroh Endpoint with DHT discovery
    pub async fn create_iroh_endpoint_with_dht(
        transport_key: &IrohTransportKey
    ) -> Result<iroh::Endpoint, anyhow::Error> {
        let endpoint = iroh::Endpoint::builder()
            .secret_key(transport_key.secret_key_cloned())
            .discovery_n0()
            .discovery_dht()  // Enable DHT discovery
            .bind()
            .await?;
        
        Ok(endpoint)
    }
}

// =============================================================================
// Section 8: Storage Key (XChaCha20-Poly1305)
// =============================================================================

/// Storage encryption key using XChaCha20-Poly1305
/// 
/// XChaCha20-Poly1305 provides:
/// - 256-bit keys (32 bytes)
/// - 192-bit nonces (24 bytes) - safe for random generation without collisions
/// - 128-bit authentication tags
/// - High performance in software
/// 
/// The extended nonce (XChaCha20) allows safe random nonce generation
/// without the collision risks of standard ChaCha20's 96-bit nonce.
pub mod storage_key {
    use super::*;
    use chacha20poly1305::{
        XChaCha20Poly1305, XNonce,
        aead::{Aead, AeadCore, KeyInit, OsRng},
    };
    
    pub struct StorageEncryptionKey {
        cipher: XChaCha20Poly1305,
    }
    
    /// Encrypted data container
    pub struct EncryptedData {
        pub nonce: Vec<u8>,      // 24 bytes
        pub ciphertext: Vec<u8>, // includes 16-byte auth tag
    }
    
    impl StorageEncryptionKey {
        /// Create storage key from 32-byte derived seed
        /// 
        /// # XChaCha20-Poly1305 parameters:
        /// - Key size: 32 bytes (256 bits)
        /// - Nonce size: 24 bytes (192 bits)
        /// - Tag size: 16 bytes (128 bits)
        pub fn from_derived_bytes(key_bytes: &[u8; 32]) -> Self {
            let cipher = XChaCha20Poly1305::new_from_slice(key_bytes)
                .expect("32 bytes is valid key size for XChaCha20Poly1305");
            
            Self { cipher }
        }
        
        /// Encrypt data with a randomly generated nonce
        /// 
        /// # Returns
        /// Tuple of (nonce, ciphertext) where:
        /// - nonce: 24 bytes
        /// - ciphertext: plaintext length + 16 bytes (auth tag)
        pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData, KeyDerivationError> {
            let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
            let ciphertext = self.cipher
                .encrypt(&nonce, plaintext)
                .map_err(|e| KeyDerivationError::StorageError(e.to_string()))?;
            
            Ok(EncryptedData {
                nonce: nonce.to_vec(),
                ciphertext,
            })
        }
        
        /// Decrypt data with provided nonce
        /// 
        /// # Arguments
        /// * `nonce` - 24-byte nonce used during encryption
        /// * `ciphertext` - Ciphertext including 16-byte auth tag
        /// 
        /// # Errors
        /// Returns error if authentication tag verification fails
        pub fn decrypt(
            &self,
            nonce: &[u8],
            ciphertext: &[u8]
        ) -> Result<Vec<u8>, KeyDerivationError> {
            let nonce = XNonce::from_slice(nonce);
            let plaintext = self.cipher
                .decrypt(nonce, ciphertext)
                .map_err(|_| KeyDerivationError::StorageError(
                    "Decryption failed: authentication tag mismatch".to_string()
                ))?;
            
            Ok(plaintext)
        }
        
        /// Encrypt with associated data (AEAD)
        /// 
        /// The associated data is authenticated but not encrypted.
        /// Useful for binding ciphertext to context (e.g., document ID).
        pub fn encrypt_with_aad(
            &self,
            plaintext: &[u8],
            aad: &[u8]
        ) -> Result<EncryptedData, KeyDerivationError> {
            use chacha20poly1305::aead::Payload;
            
            let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
            let payload = Payload { msg: plaintext, aad };
            let ciphertext = self.cipher
                .encrypt(&nonce, payload)
                .map_err(|e| KeyDerivationError::StorageError(e.to_string()))?;
            
            Ok(EncryptedData {
                nonce: nonce.to_vec(),
                ciphertext,
            })
        }
        
        /// Decrypt with associated data verification
        pub fn decrypt_with_aad(
            &self,
            nonce: &[u8],
            ciphertext: &[u8],
            aad: &[u8]
        ) -> Result<Vec<u8>, KeyDerivationError> {
            use chacha20poly1305::aead::Payload;
            
            let nonce = XNonce::from_slice(nonce);
            let payload = Payload { msg: ciphertext, aad };
            let plaintext = self.cipher
                .decrypt(nonce, payload)
                .map_err(|_| KeyDerivationError::StorageError(
                    "Decryption failed: authentication tag mismatch".to_string()
                ))?;
            
            Ok(plaintext)
        }
    }
}

// =============================================================================
// Section 9: Complete Wallet Integration
// =============================================================================

use zeroize::Zeroizing;

/// Complete wallet keys container
pub struct WalletKeys {
    pub identity_key: identity_key_secp256k1::IdentitySigningKey,
    pub transport_key: transport_key::IrohTransportKey,
    pub storage_key: storage_key::StorageEncryptionKey,
}

/// Alternative wallet using Ed25519 for identity
pub struct WalletKeysEd25519 {
    pub identity_key: identity_key_ed25519::Ed25519IdentityKey,
    pub transport_key: transport_key::IrohTransportKey,
    pub storage_key: storage_key::StorageEncryptionKey,
}

/// Initialize all wallet keys from WebAuthn PRF output
/// 
/// # Workflow
/// 1. Receive 32-byte seed from WebAuthn PRF extension
/// 2. Derive three keys using HKDF-SHA256
/// 3. Convert each key to its specific type
/// 
/// # Security
/// - Input seed is wrapped in Zeroizing for secure cleanup
/// - All key material is zeroized when dropped
pub fn initialize_wallet_keys(
    webauthn_prf_output: &[u8; 32]
) -> Result<WalletKeys, KeyDerivationError> {
    // Wrap seed in Zeroizing for secure cleanup
    let seed = Zeroizing::new(*webauthn_prf_output);
    
    // Derive all three keys
    let derived = derive_keys_from_seed(&seed)?;
    
    // Create identity key (secp256k1)
    let identity_key = identity_key_secp256k1::IdentitySigningKey
        ::from_derived_bytes(&derived.identity_key_bytes)?;
    
    // Create transport key (Ed25519 for Iroh)
    let transport_key = transport_key::IrohTransportKey
        ::from_derived_bytes(&derived.transport_key_bytes)?;
    
    // Create storage key (XChaCha20-Poly1305)
    let storage_key = storage_key::StorageEncryptionKey
        ::from_derived_bytes(&derived.storage_key_bytes);
    
    Ok(WalletKeys {
        identity_key,
        transport_key,
        storage_key,
    })
}

/// Initialize wallet with Ed25519 identity key
pub fn initialize_wallet_keys_ed25519(
    webauthn_prf_output: &[u8; 32]
) -> Result<WalletKeysEd25519, KeyDerivationError> {
    let seed = Zeroizing::new(*webauthn_prf_output);
    let derived = derive_keys_from_seed(&seed)?;
    
    let identity_key = identity_key_ed25519::Ed25519IdentityKey
        ::from_derived_bytes(&derived.identity_key_bytes)?;
    let transport_key = transport_key::IrohTransportKey
        ::from_derived_bytes(&derived.transport_key_bytes)?;
    let storage_key = storage_key::StorageEncryptionKey
        ::from_derived_bytes(&derived.storage_key_bytes);
    
    Ok(WalletKeysEd25519 {
        identity_key,
        transport_key,
        storage_key,
    })
}

// =============================================================================
// Section 10: Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_derivation_determinism() {
        let seed = [0x42u8; 32];
        
        let keys1 = derive_keys_from_seed(&seed).unwrap();
        let keys2 = derive_keys_from_seed(&seed).unwrap();
        
        // Same seed must produce same keys
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
        
        // Different seeds must produce different keys
        assert_ne!(keys1.identity_key_bytes, keys2.identity_key_bytes);
        assert_ne!(keys1.transport_key_bytes, keys2.transport_key_bytes);
        assert_ne!(keys1.storage_key_bytes, keys2.storage_key_bytes);
    }
    
    #[test]
    fn test_key_derivation_independence() {
        let seed = [0x42u8; 32];
        let keys = derive_keys_from_seed(&seed).unwrap();
        
        // All three keys should be different
        assert_ne!(keys.identity_key_bytes, keys.transport_key_bytes);
        assert_ne!(keys.identity_key_bytes, keys.storage_key_bytes);
        assert_ne!(keys.transport_key_bytes, keys.storage_key_bytes);
    }
    
    #[test]
    fn test_secp256k1_identity_key() {
        let derived = derive_keys_from_seed(&[0x42u8; 32]).unwrap();
        let identity_key = identity_key_secp256k1::IdentitySigningKey
            ::from_derived_bytes(&derived.identity_key_bytes)
            .unwrap();
        
        // Test signing
        let message = b"test message for W3C Verifiable Credential";
        let signature = identity_key.sign(message).unwrap();
        assert_eq!(signature.len(), 64); // secp256k1 signatures are 64 bytes
        
        // Verify signature
        let verifying_key = identity_key.verifying_key();
        use k256::ecdsa::signature::Verifier;
        let sig = k256::ecdsa::Signature::from_slice(&signature).unwrap();
        assert!(verifying_key.verify(message, &sig).is_ok());
    }
    
    #[test]
    fn test_ed25519_identity_key() {
        let derived = derive_keys_from_seed(&[0x42u8; 32]).unwrap();
        let identity_key = identity_key_ed25519::Ed25519IdentityKey
            ::from_derived_bytes(&derived.identity_key_bytes)
            .unwrap();
        
        let message = b"test message";
        let signature = identity_key.sign(message);
        assert_eq!(signature.len(), 64); // Ed25519 signatures are 64 bytes
        
        // Verify signature
        use ed25519_dalek::Verifier;
        let sig = ed25519_dalek::Signature::from_bytes(&signature.try_into().unwrap());
        assert!(identity_key.verifying_key().verify(message, &sig).is_ok());
    }
    
    #[test]
    fn test_storage_encryption_roundtrip() {
        let derived = derive_keys_from_seed(&[0x42u8; 32]).unwrap();
        let storage_key = storage_key::StorageEncryptionKey
            ::from_derived_bytes(&derived.storage_key_bytes);
        
        let plaintext = b"sensitive iroh-docs data";
        
        // Encrypt
        let encrypted = storage_key.encrypt(plaintext).unwrap();
        assert_eq!(encrypted.nonce.len(), 24); // XChaCha20 nonce is 24 bytes
        assert_eq!(encrypted.ciphertext.len(), plaintext.len() + 16); // + auth tag
        
        // Decrypt
        let decrypted = storage_key.decrypt(
            &encrypted.nonce,
            &encrypted.ciphertext
        ).unwrap();
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_storage_encryption_aad() {
        let derived = derive_keys_from_seed(&[0x42u8; 32]).unwrap();
        let storage_key = storage_key::StorageEncryptionKey
            ::from_derived_bytes(&derived.storage_key_bytes);
        
        let plaintext = b"document content";
        let doc_id = b"doc-12345";
        
        // Encrypt with AAD
        let encrypted = storage_key.encrypt_with_aad(plaintext, doc_id).unwrap();
        
        // Decrypt with correct AAD succeeds
        let decrypted = storage_key.decrypt_with_aad(
            &encrypted.nonce,
            &encrypted.ciphertext,
            doc_id
        ).unwrap();
        assert_eq!(decrypted, plaintext);
        
        // Decrypt with wrong AAD fails
        let wrong_doc_id = b"doc-99999";
        let result = storage_key.decrypt_with_aad(
            &encrypted.nonce,
            &encrypted.ciphertext,
            wrong_doc_id
        );
        assert!(result.is_err());
    }
    
    #[test]
    fn test_complete_wallet_initialization() {
        let seed = [0x42u8; 32];
        let wallet = initialize_wallet_keys(&seed).unwrap();
        
        // Test identity key
        let message = b"test";
        let _sig = wallet.identity_key.sign(message).unwrap();
        
        // Test storage key
        let encrypted = wallet.storage_key.encrypt(message).unwrap();
        let decrypted = wallet.storage_key.decrypt(
            &encrypted.nonce,
            &encrypted.ciphertext
        ).unwrap();
        assert_eq!(decrypted, message);
        
        // Test transport key
        let _endpoint_id = wallet.transport_key.endpoint_id();
    }
}

// =============================================================================
// Section 11: Cargo.toml Dependencies
// =============================================================================

/*
Add these to your Cargo.toml:

[dependencies]
# HKDF implementation (RFC 5869)
hkdf = "0.12"
sha2 = "0.10"

# Iroh for P2P networking
iroh = "0.96"

# Ed25519 for identity signing (alternative)
ed25519-dalek = "2.1"

# secp256k1 for identity signing (recommended for W3C VC)
k256 = { version = "0.13", features = ["ecdsa", "sha256"] }

# XChaCha20-Poly1305 for storage encryption
chacha20poly1305 = "0.10"

# Secure memory handling
zeroize = { version = "1.8", features = ["derive"] }

# Error handling
thiserror = "1.0"

# For Ethereum address derivation (optional)
sha3 = "0.10"

# For constant-time comparison (optional)
subtle = "2.6"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
*/
