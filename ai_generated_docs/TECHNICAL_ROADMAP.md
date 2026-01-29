# Local-First Identity Wallet: Technical Implementation Roadmap

## Comprehensive Architecture for Iroh-Based P2P Identity Wallet with WebAuthn PRF

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Cryptographic Foundation](#cryptographic-foundation)
4. [WebAuthn PRF Integration](#webauthn-prf-integration)
5. [Iroh Node Initialization](#iroh-node-initialization)
6. [Mobile Runtime Architecture](#mobile-runtime-architecture)
7. [Capability Delegation Protocol](#capability-delegation-protocol)
8. [Stateless Recovery Flow](#stateless-recovery-flow)
9. [Fallback Strategies](#fallback-strategies)
10. [Implementation Roadmap](#implementation-roadmap)
11. [References](#references)

---

## Executive Summary

This document provides a comprehensive technical implementation plan for a mobile-first "Local-First" identity wallet that combines:

- **Stateless Key Management**: WebAuthn PRF (Passkeys) for on-demand key derivation
- **Hybrid Key Hierarchy**: HKDF-SHA256 for deriving Identity, Transport, and Storage keys
- **P2P Networking**: Iroh for peer-to-peer data synchronization
- **Capability Security**: UCAN-based delegation between Identity and Transport keys
- **Cloud-Agnostic Recovery**: Stateless recovery via JoFin Cloud Vault

### Key Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Zero Key Storage** | Private keys derived on-demand via WebAuthn PRF |
| **Deterministic Recovery** | Same Passkey + Same Salt = Same Keys |
| **Key Separation** | Identity Key for signing, Transport Key for networking |
| **Capability Delegation** | UCAN tokens delegate sync authority without exposing root key |
| **CRDT Synchronization** | iroh-docs with trust:/claim:/pointer: prefixes |

---

## Architecture Overview

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                          LOCAL-FIRST IDENTITY WALLET                             │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │                         PRESENTATION LAYER                                │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │  │
│  │  │  React      │  │   Identity  │  │   Vault     │  │   Settings      │  │  │
│  │  │  Native UI  │  │   Manager   │  │   Sync      │  │   & Recovery    │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────────┘  │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                    │                                             │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │                      REACT NATIVE BRIDGE (UniFFi)                         │  │
│  │  ┌─────────────────────────────────────────────────────────────────────┐ │  │
│  │  │  TypeScript Interface  │  JSI C++  │  Rust FFI (uniffi-bindgen-rn)   │ │  │
│  │  └─────────────────────────────────────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                    │                                             │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │                         RUST CORE LAYER                                   │  │
│  │  ┌───────────────┐ ┌───────────────┐ ┌───────────────┐ ┌──────────────┐ │  │
│  │  │ Key Derivation│ │  Iroh Node    │ │  Capability   │ │   Storage    │ │  │
│  │  │ (HKDF/PRF)    │ │  (P2P/QUIC)   │ │  (UCAN)       │ │ (Encrypted)  │ │  │
│  │  └───────────────┘ └───────────────┘ └───────────────┘ └──────────────┘ │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                    │                                             │
│  ┌──────────────────────────────────────────────────────────────────────────┐  │
│  │                      PLATFORM SERVICES                                    │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │  │
│  │  │   iOS:      │  │  Android:   │  │  WebAuthn   │  │   Cloud Relay   │  │  │
│  │  │ PushKit/    │  │  FCM/       │  │  (Platform  │  │   (DERP)        │  │  │
│  │  │ BGTask      │  │  Foreground │  │  Auth)      │  │                 │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────────┘  │  │
│  └──────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Key Hierarchy

```
WebAuthn PRF Output (32 bytes)
         │
         ▼
┌─────────────────┐
│  HKDF-SHA256    │
│  Extract/Expand │
└────────┬────────┘
         │
    ┌────┼────┬────────────┐
    │    │    │            │
    ▼    ▼    ▼            ▼
┌──────┐ ┌──────┐ ┌──────────┐ ┌──────────┐
│Master│ │Trans-│ │ Identity │ │ Storage  │
│Entro-│ │port  │ │  Key     │ │  Key     │
│py    │ │Key   │ │(secp256k1│ │(XChaCha20│
│      │ │(Ed255│ │ or Ed25519│ │-Poly1305)│
└──┬───┘ └──┬───┘ └────┬─────┘ └────┬─────┘
   │        │          │            │
   │        │          ▼            ▼
   │        │    ┌──────────┐  ┌──────────┐
   │        │    │ W3C VC   │  │ iroh-docs│
   │        │    │ Signing  │  │ Encrypt  │
   │        │    └──────────┘  └──────────┘
   │        │
   │        ▼
   │   ┌──────────┐
   │   │ Iroh     │
   │   │ Endpoint │
   │   │ (Node ID)│
   │   └────┬─────┘
   │        │
   │        ▼
   │   ┌──────────┐
   └──►│ iroh-docs│
       │ Sync     │
       └──────────┘
```

---

## Cryptographic Foundation

### HKDF-SHA256 Parameters (RFC 5869)

#### Salt Strategy

| Parameter | Value | Notes |
|-----------|-------|-------|
| **Application Salt** | `b"LocalFirstIdentityWallet_v1_2024"` | 32 bytes, non-secret, domain separation |
| **Hash Function** | SHA-256 | 32-byte output |
| **IKM** | 32-byte PRF result | From WebAuthn PRF extension |

#### Info Strings for Key Derivation

| Key Type | Info String | Output Length | Algorithm |
|----------|-------------|---------------|-----------|
| **Identity Key** | `"identity-wallet/v1/identity-key"` | 32 bytes | secp256k1 or Ed25519 |
| **Transport Key** | `"identity-wallet/v1/transport-key"` | 32 bytes | Ed25519 (Iroh) |
| **Storage Key (DEK)** | `"identity-wallet/v1/storage-key"` | 32 bytes | XChaCha20-Poly1305 |

### Rust Implementation: Key Derivation

```rust
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};
use thiserror::Error;

/// Application-specific salt for domain separation (RFC 5869 Section 3.1)
pub const APP_SALT: &[u8] = b"LocalFirstIdentityWallet_v1_2024";

/// Info strings for key derivation (RFC 5869 Section 3.2)
pub const INFO_IDENTITY_KEY: &[u8] = b"identity-wallet/v1/identity-key";
pub const INFO_TRANSPORT_KEY: &[u8] = b"identity-wallet/v1/transport-key";
pub const INFO_STORAGE_KEY: &[u8] = b"identity-wallet/v1/storage-key";

/// Output key lengths
pub const IDENTITY_KEY_LEN: usize = 32;
pub const TRANSPORT_KEY_LEN: usize = 32;
pub const STORAGE_KEY_LEN: usize = 32;

#[derive(Error, Debug)]
pub enum KeyDerivationError {
    #[error("HKDF expansion failed: {0}")]
    HkdfError(#[from] hkdf::InvalidLength),
    
    #[error("Invalid master seed length: expected 32 bytes, got {0}")]
    InvalidSeedLength(usize),
}

/// Container for all derived keys with automatic zeroization
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DerivedKeys {
    #[zeroize(skip)]
    pub identity_key_bytes: [u8; IDENTITY_KEY_LEN],
    #[zeroize(skip)]
    pub transport_key_bytes: [u8; TRANSPORT_KEY_LEN],
    pub storage_key_bytes: [u8; STORAGE_KEY_LEN],
}

/// Derive all three keys from a 32-byte master seed using HKDF-SHA256
/// 
/// # Algorithm (RFC 5869)
/// 1. **Extract**: PRK = HMAC-SHA256(salt, IKM)
/// 2. **Expand**: OKM = HKDF-Expand(PRK, info, L)
pub fn derive_keys_from_seed(
    master_seed: &[u8; 32]
) -> Result<DerivedKeys, KeyDerivationError> {
    // Step 1: HKDF Extract with application salt
    let hkdf = Hkdf::<Sha256>::new(Some(APP_SALT), master_seed);
    
    // Step 2: HKDF Expand for each key with unique info strings
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
```

### Identity Key Implementation (secp256k1)

```rust
use k256::ecdsa::{SigningKey, signature::Signer};
use k256::SecretKey;

/// Identity signing key using secp256k1 for W3C Verifiable Credentials
pub struct IdentitySigningKey {
    signing_key: SigningKey,
}

impl IdentitySigningKey {
    /// Create identity key from 32-byte derived seed
    pub fn from_derived_bytes(key_bytes: &[u8; 32]) -> Result<Self, KeyDerivationError> {
        let secret_key = SecretKey::from_bytes(key_bytes.into())?;
        let signing_key = SigningKey::from(secret_key);
        Ok(Self { signing_key })
    }
    
    /// Sign a message (for W3C Verifiable Credentials)
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, KeyDerivationError> {
        let signature: k256::ecdsa::Signature = self.signing_key.sign(message);
        Ok(signature.to_bytes().to_vec())
    }
    
    /// Get compressed public key bytes (33 bytes)
    pub fn public_key_compressed(&self) -> Vec<u8> {
        self.signing_key.verifying_key()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec()
    }
}
```

### Transport Key Implementation (Iroh Ed25519)

```rust
/// Iroh-compatible transport key for P2P networking
pub struct IrohTransportKey {
    secret_key: iroh::SecretKey,
}

impl IrohTransportKey {
    /// Create Iroh transport key from 32-byte derived seed
    /// 
    /// # Iroh Compatibility
    /// - Iroh SecretKey is exactly 32 bytes (Ed25519 seed)
    /// - Public key derived deterministically from seed
    /// - Used as EndpointId in iroh-net
    pub fn from_derived_bytes(key_bytes: &[u8; 32]) -> Result<Self, KeyDerivationError> {
        let secret_key = iroh::SecretKey::from_bytes(key_bytes);
        Ok(Self { secret_key })
    }
    
    /// Get the Iroh SecretKey for use with Endpoint builder
    pub fn secret_key(&self) -> &iroh::SecretKey {
        &self.secret_key
    }
    
    /// Get the endpoint ID (public key) for this transport key
    pub fn endpoint_id(&self) -> iroh::EndpointId {
        self.secret_key.public()
    }
}

/// Create an Iroh Endpoint with the derived transport key
pub async fn create_iroh_endpoint(
    transport_key: &IrohTransportKey
) -> Result<iroh::Endpoint, anyhow::Error> {
    let endpoint = iroh::Endpoint::builder()
        .secret_key(transport_key.secret_key().clone())
        .discovery_n0()  // Use N0 discovery service
        .bind()
        .await?;
    
    Ok(endpoint)
}
```

### Storage Key Implementation (XChaCha20-Poly1305)

```rust
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};

/// Storage encryption key for iroh-docs data
pub struct StorageEncryptionKey {
    cipher: XChaCha20Poly1305,
}

pub struct EncryptedData {
    pub nonce: Vec<u8>,      // 24 bytes
    pub ciphertext: Vec<u8>, // includes 16-byte auth tag
}

impl StorageEncryptionKey {
    /// Create storage key from 32-byte derived seed
    pub fn from_derived_bytes(key_bytes: &[u8; 32]) -> Self {
        let cipher = XChaCha20Poly1305::new_from_slice(key_bytes)
            .expect("32 bytes is valid key size for XChaCha20Poly1305");
        Self { cipher }
    }
    
    /// Encrypt data with a randomly generated nonce
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData, KeyDerivationError> {
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self.cipher.encrypt(&nonce, plaintext)
            .map_err(|e| KeyDerivationError::StorageError(e.to_string()))?;
        
        Ok(EncryptedData {
            nonce: nonce.to_vec(),
            ciphertext,
        })
    }
    
    /// Decrypt data with provided nonce
    pub fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, KeyDerivationError> {
        let nonce = XNonce::from_slice(nonce);
        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|_| KeyDerivationError::StorageError(
                "Decryption failed: authentication tag mismatch".to_string()
            ))?;
        Ok(plaintext)
    }
}
```

---

## WebAuthn PRF Integration

### PRF Extension Overview

The WebAuthn PRF (Pseudo-Random Function) extension allows extracting a deterministic 32-byte secret from a Passkey credential.

| Property | Value |
|----------|-------|
| **Output Size** | 32 bytes (256 bits) |
| **Determinism** | Same input → Same output for same credential |
| **Hardware Backing** | CTAP2 `hmac-secret` extension (TPM/Secure Enclave) |
| **User Gesture** | Required (touch, PIN, biometric) |

### JavaScript Implementation

#### Credential Creation (Enable PRF)

```javascript
const createOptions = {
  publicKey: {
    rp: { name: "LocalFirst Identity Wallet", id: "wallet.example.com" },
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
// Per-credential salt (stored with cloud vault metadata)
const credentialSalt = new Uint8Array([/* 32 bytes stored per credential */]);

const getOptions = {
  publicKey: {
    challenge: crypto.getRandomValues(new Uint8Array(32)),
    rpId: "wallet.example.com",
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

### PRF Support Matrix (2025)

| Platform | Browser | Platform Authenticator | Roaming Authenticator |
|----------|---------|----------------------|---------------------|
| iOS 18+ | Safari/Chrome | ✅ PRF Supported | ❌ Not Supported |
| Android | Chrome | ✅ PRF Supported | ✅ USB: Yes, NFC: No |
| macOS 15+ | Safari/Chrome | ✅ PRF Supported | ✅ Chrome: Yes |
| Windows 11 | Chrome/Edge | ❌ No hmac-secret | ✅ Supported |

---

## Iroh Node Initialization

### Complete Initialization Sequence

```rust
use iroh::{Endpoint, RelayMode};
use iroh_docs::{Doc, Author, NamespaceSecret};

/// Complete wallet initialization from WebAuthn PRF output
pub async fn initialize_wallet(
    webauthn_prf_output: &[u8; 32]
) -> Result<IdentityWallet, WalletError> {
    // 1. Derive all keys using HKDF-SHA256
    let derived = derive_keys_from_seed(webauthn_prf_output)
        .map_err(|e| WalletError::KeyDerivation(e.to_string()))?;
    
    // 2. Create Identity Key (secp256k1 for W3C VC compatibility)
    let identity_key = IdentitySigningKey::from_derived_bytes(&derived.identity_key_bytes)
        .map_err(|e| WalletError::IdentityKey(e.to_string()))?;
    
    // 3. Create Transport Key (Ed25519 for Iroh)
    let transport_key = IrohTransportKey::from_derived_bytes(&derived.transport_key_bytes)
        .map_err(|e| WalletError::TransportKey(e.to_string()))?;
    
    // 4. Create Storage Key (XChaCha20-Poly1305)
    let storage_key = StorageEncryptionKey::from_derived_bytes(&derived.storage_key_bytes);
    
    // 5. Initialize Iroh Endpoint with derived transport key
    let endpoint = iroh::Endpoint::builder()
        .secret_key(transport_key.secret_key().clone())
        .relay_mode(RelayMode::Enabled)
        .discovery_n0()
        .bind()
        .await
        .map_err(|e| WalletError::Endpoint(e.to_string()))?;
    
    // 6. Create iroh-docs namespace (will be delegated via UCAN)
    let namespace = NamespaceSecret::new(&mut rand::thread_rng());
    
    // 7. Create Author from transport key for signing entries
    let author = Author::from(transport_key.secret_key().clone());
    
    Ok(IdentityWallet {
        identity_key,
        transport_key,
        storage_key,
        endpoint,
        namespace,
        author,
    })
}

pub struct IdentityWallet {
    pub identity_key: IdentitySigningKey,
    pub transport_key: IrohTransportKey,
    pub storage_key: StorageEncryptionKey,
    pub endpoint: iroh::Endpoint,
    pub namespace: NamespaceSecret,
    pub author: Author,
}
```

### Node ID Consistency

The critical property for recovery is that the **same Transport Key produces the same Node ID**:

```rust
// On Device A
let transport_key_a = IrohTransportKey::from_derived_bytes(&seed)?;
let endpoint_a = create_iroh_endpoint(&transport_key_a).await?;
let node_id_a = endpoint_a.node_id();

// On Device B (same seed)
let transport_key_b = IrohTransportKey::from_derived_bytes(&seed)?;
let endpoint_b = create_iroh_endpoint(&transport_key_b).await?;
let node_id_b = endpoint_b.node_id();

// node_id_a == node_id_b (deterministic)
assert_eq!(node_id_a, node_id_b);
```

---

## Mobile Runtime Architecture

### UniFFi Integration for React Native

Mozilla's `uniffi-bindgen-react-native` provides production-ready React Native Turbo Module generation.

#### UDL Interface Definition

```idl
// iroh_wallet.udl
namespace iroh_wallet {
    [Async]
    NodeHandle init_node(NodeConfig config);
    
    NodeInfo get_node_info(NodeHandle handle);
    
    [Async]
    void shutdown_node(NodeHandle handle);
};

dictionary NodeConfig {
    sequence<string> relay_urls;
    boolean enable_discovery;
    string? alpn_protocol;
};

dictionary NodeInfo {
    string node_id;
    sequence<string> relay_addresses;
    boolean is_connected;
};

interface NodeHandle {};

[Error]
enum WalletError {
    "NodeInitFailed",
    "ConnectionFailed",
    "InvalidKey",
    "NetworkError"
};
```

#### Rust Implementation with Proc Macros

```rust
use std::sync::Arc;
use iroh::Endpoint;
use tokio::sync::RwLock;

uniffi::setup_scaffolding!();

#[derive(uniffi::Record, Clone, Debug)]
pub struct NodeConfig {
    pub relay_urls: Vec<String>,
    pub enable_discovery: bool,
}

#[derive(uniffi::Object)]
pub struct NodeHandle {
    endpoint: Arc<RwLock<Endpoint>>,
}

#[uniffi::export(async_runtime = "tokio")]
impl NodeHandle {
    #[uniffi::constructor]
    pub async fn new(config: NodeConfig) -> Result<Arc<Self>, WalletError> {
        let endpoint = Endpoint::builder()
            .discovery(config.enable_discovery)
            .bind()
            .await
            .map_err(|e| WalletError::NodeInitFailed(e.to_string()))?;
        
        Ok(Arc::new(Self {
            endpoint: Arc::new(RwLock::new(endpoint)),
        }))
    }
    
    pub async fn shutdown(&self) -> Result<(), WalletError> {
        let endpoint = self.endpoint.read().await;
        endpoint.close().await;
        Ok(())
    }
}
```

### Background Execution Strategy

#### iOS Background Modes

```xml
<!-- Info.plist -->
<key>UIBackgroundModes</key>
<array>
    <!-- For instant wake on sign requests -->
    <string>voip</string>
    <!-- For background sync -->
    <string>fetch</string>
    <!-- For silent push notifications -->
    <string>remote-notification</string>
</array>
```

#### Android Foreground Service

```kotlin
// AndroidManifest.xml
<service
    android:name=".IrohP2PService"
    android:enabled="true"
    android:exported="false"
    android:foregroundServiceType="dataSync" />

// IrohP2PService.kt
class IrohP2PService : Service() {
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> startP2P()
            ACTION_STOP -> stopP2P()
        }
        return START_STICKY
    }
    
    private fun startP2P() {
        startForeground(NOTIFICATION_ID, createNotification())
        serviceScope.launch {
            IrohWalletModule.initializeNode()
        }
    }
}
```

### Wake-and-Sign Pattern

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│   Signer     │         │   Cloud      │         │   Wallet     │
│   (Offline)  │         │   Relay      │         │   (Online)   │
└──────┬───────┘         └──────┬───────┘         └──────┬───────┘
       │                        │                        │
       │  1. Queue sign request │                        │
       │◄───────────────────────┤                        │
       │                        │                        │
       │                        │  2. Send push (FCM/APNs)│
       │                        ├───────────────────────►│
       │                        │                        │
       │                        │  3. Wake up            │
       │                        │◄───────────────────────┤
       │                        │                        │
       │  4. Connect via relay  │                        │
       │◄───────────────────────┤                        │
       │                        │                        │
       │  5. Sign & return      │                        │
       ├───────────────────────►│                        │
       │                        │                        │
       │                        │  6. Return signature   │
       │                        ├───────────────────────►│
```

---

## Capability Delegation Protocol

### UCAN-Based Delegation

UCANs (User Controlled Authorization Networks) provide self-certifying, delegable, time-bounded capabilities.

#### Delegation Token Structure

```json
{
  "ucv": "1.0.0-rc.1",
  "iss": "did:key:z6Mk...IdentityKey...",
  "aud": "did:key:z6Mki...TransportKey...",
  "sub": "did:key:z6Mkq...IdentityKey...",
  "nbf": 1704137004,
  "exp": 1706745600,
  "att": [
    {
      "with": "iroh:namespace:z6Mkw...NamespaceId...",
      "can": "iroh/docs/write",
      "nb": {
        "key_prefix": ["trust:", "claim:", "pointer:"],
        "max_entries": 10000
      }
    },
    {
      "with": "iroh:namespace:z6Mkw...NamespaceId...",
      "can": "iroh/docs/read",
      "nb": {
        "key_prefix": ["trust:", "claim:", "pointer:"]
      }
    },
    {
      "with": "iroh:namespace:z6Mkw...NamespaceId...",
      "can": "iroh/docs/sync",
      "nb": {
        "peers": ["*"],
        "relay": true
      }
    }
  ],
  "prf": []
}
```

### Rust Implementation: UCAN Delegation

```rust
use ucan::{Ucan, UcanBuilder};
use iroh_docs::{Author, NamespaceSecret};
use ed25519_dalek::SigningKey;

/// Creates a UCAN delegation from Identity Key to Transport Key
pub fn create_delegation_ucan(
    identity_key: &SigningKey,
    transport_key: &SigningKey,
    namespace: &NamespaceSecret,
    validity_hours: u64,
) -> Result<Ucan, DelegationError> {
    let identity_did = did_from_signing_key(identity_key);
    let transport_did = did_from_signing_key(transport_key);
    let namespace_id = namespace.id();
    
    let now = current_timestamp();
    let expiry = now + (validity_hours * 3600);
    
    let capabilities = vec![
        Capability {
            with: format!("iroh:namespace:{}", namespace_id),
            can: "iroh/docs/write".to_string(),
            nb: serde_json::json!({
                "key_prefix": ["trust:", "claim:", "pointer:"],
                "max_entries": 10000,
            }),
        },
        Capability {
            with: format!("iroh:namespace:{}", namespace_id),
            can: "iroh/docs/read".to_string(),
            nb: serde_json::json!({
                "key_prefix": ["trust:", "claim:", "pointer:"],
            }),
        },
    ];
    
    let ucan = UcanBuilder::default()
        .issued_by(&identity_did)
        .for_audience(&transport_did)
        .with_subject(&identity_did)
        .not_before(now)
        .expires_at(expiry)
        .with_capabilities(capabilities)
        .sign(identity_key)?;
    
    Ok(ucan)
}
```

### CRDT Data Model for ISD

| Prefix | CRDT Type | Purpose |
|--------|-----------|---------|
| `trust:` | LWW-Register | Trust scores per context/metric |
| `claim:` | OR-Set | W3C Verifiable Credentials |
| `pointer:` | MV-Register | External service references |

```rust
/// Key prefix structure for Identity as Social Intersection
pub enum KeyPrefix {
    /// Format: trust:{target_did}:{context}:{metric}
    Trust { target: String, context: String, metric: String },
    
    /// Format: claim:{credential_type}:{credential_id}:{property}
    Claim { cred_type: String, cred_id: String, property: String },
    
    /// Format: pointer:{reference_type}:{entity_id}:{property}
    Pointer { ref_type: String, entity_id: String, property: String },
}

impl KeyPrefix {
    pub fn to_key(&self) -> String {
        match self {
            KeyPrefix::Trust { target, context, metric } => {
                format!("trust:{}:{}:{}", target, context, metric)
            }
            KeyPrefix::Claim { cred_type, cred_id, property } => {
                format!("claim:{}:{}:{}", cred_type, cred_id, property)
            }
            KeyPrefix::Pointer { ref_type, entity_id, property } => {
                format!("pointer:{}:{}:{}", ref_type, entity_id, property)
            }
        }
    }
}
```

---

## Stateless Recovery Flow

### Recovery Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           RECOVERY ARCHITECTURE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐         ┌──────────────┐         ┌──────────────────────┐ │
│  │   New Device │         │   Passkey    │         │   Master Entropy     │ │
│  │  (User)      │◄───────►│  Provider    │◄───────►│   Derivation         │ │
│  │              │  Auth   │ (iCloud/GPM) │  PRF    │   (HKDF)             │ │
│  └──────────────┘         └──────────────┘         └──────────────────────┘ │
│         │                                                        │          │
│         │         ┌──────────────┐         ┌──────────────────────┐         │
│         └────────►│  Transport   │◄───────►│   Iroh Node          │         │
│                   │  Key Gen     │         │   Initialization     │         │
│                   └──────────────┘         └──────────────────────┘         │
│                          │                            │                     │
│                          ▼                            ▼                     │
│                   ┌──────────────────────────────────────────┐              │
│                   │         JoFin Cloud Vault Peer           │              │
│                   │  ┌──────────────┐    ┌──────────────┐    │              │
│                   │  │  Encrypted   │    │   Peer       │    │              │
│                   │  │  Identity    │    │   Discovery  │    │              │
│                   │  │  History     │    │   (DNS/DHT)  │    │              │
│                   │  └──────────────┘    └──────────────┘    │              │
│                   └──────────────────────────────────────────┘              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Recovery Sequence (7 Phases)

| Phase | Action | Output |
|-------|--------|--------|
| 1 | Passkey Authentication | User verified, challenge signed |
| 2 | PRF Seed Derivation | 32-byte secrets per salt |
| 3 | Master Entropy Re-derivation | 256-bit root key |
| 4 | Transport Key Regeneration | Ed25519 key pair |
| 5 | Iroh Node Initialization | Active endpoint with consistent ID |
| 6 | Cloud Vault Peer Discovery | Vault addressing info |
| 7 | Encrypted Data Sync | Decrypted identity bundle |

### TypeScript Recovery Implementation

```typescript
// recovery.ts - Complete stateless recovery implementation

interface RecoveryConfig {
  vaultEndpointId: string;
  vaultRelayUrl?: string;
  discoveryServices: ('dns' | 'pkarr' | 'dht')[];
}

interface RecoveryResult {
  success: boolean;
  identity: IdentityBundle;
  endpoint: iroh.Endpoint;
  error?: string;
}

/**
 * Main recovery function - stateless identity restoration
 */
export async function recoverIdentity(
  config: RecoveryConfig
): Promise<RecoveryResult> {
  try {
    // Phase 1: Authenticate with Passkey and derive PRF secrets
    const prfResults = await authenticateWithPrf();
    
    // Phase 2: Derive all cryptographic keys
    const keys = await deriveAllKeys(prfResults);
    
    // Phase 3: Initialize Iroh node with regenerated transport key
    const endpoint = await initializeIrohNode(keys.transportKeyPair);
    
    // Phase 4: Discover and connect to cloud vault
    const vaultConnection = await connectToVault(endpoint, config);
    
    // Phase 5: Fetch and decrypt identity bundle
    const identityBundle = await fetchAndDecryptIdentity(
      vaultConnection,
      keys.encryptionKey
    );
    
    // Phase 6: Verify bundle integrity
    await verifyIdentityBundle(identityBundle, keys.signingKey);
    
    return {
      success: true,
      identity: identityBundle,
      endpoint
    };
    
  } catch (error) {
    return {
      success: false,
      identity: null as any,
      endpoint: null as any,
      error: error.message
    };
  }
}

async function authenticateWithPrf(): Promise<PrfResults> {
  // Check PRF support
  const prfSupport = await detectPrfSupport();
  if (!prfSupport.prfSupported) {
    throw new Error('PRF not supported - use fallback recovery');
  }
  
  // Get stored salts from local cache or cloud metadata
  const salts = await getStoredSalts();
  
  const getOptions: PublicKeyCredentialRequestOptions = {
    challenge: await fetchChallenge(),
    rpId: 'wallet.jofin.io',
    allowCredentials: [], // Empty for discoverable credentials
    userVerification: 'required',
    extensions: {
      prf: {
        eval: {
          first: salts.master,
          second: salts.transport,
        }
      }
    }
  };
  
  const assertion = await navigator.credentials.get({ publicKey: getOptions });
  const extResults = assertion.getClientExtensionResults();
  
  return {
    master: new Uint8Array(extResults.prf.results.first),
    transport: new Uint8Array(extResults.prf.results.second),
    credentialId: assertion.id
  };
}
```

---

## Fallback Strategies

### PRF Support Detection

```javascript
async function detectPrfSupport() {
  const result = {
    prfSupported: false,
    fallbackRequired: false,
    fallbackMethod: null,
  };

  // Method 1: Check using getClientCapabilities (modern browsers)
  if (window.PublicKeyCredential && 
      typeof PublicKeyCredential.getClientCapabilities === 'function') {
    try {
      const caps = await PublicKeyCredential.getClientCapabilities();
      if (caps.extensions?.includes('prf')) {
        result.prfSupported = true;
        return result;
      }
    } catch (e) {
      console.warn('getClientCapabilities failed:', e);
    }
  }

  // Method 2: Feature detection via trial registration
  try {
    const createOptions = {
      publicKey: {
        rp: { name: 'Test', id: location.hostname },
        user: { id: new Uint8Array(16), name: 'test', displayName: 'Test' },
        challenge: crypto.getRandomValues(new Uint8Array(32)),
        pubKeyCredParams: [{ alg: -7, type: 'public-key' }],
        extensions: { prf: {} }
      }
    };
    
    const credential = await navigator.credentials.create(createOptions);
    const extResults = credential.getClientExtensionResults();
    result.prfSupported = extResults.prf?.enabled === true;
  } catch (e) {
    // Expected to fail - we only care about extension handling
  }

  if (!result.prfSupported) {
    result.fallbackRequired = true;
    result.fallbackMethod = selectFallbackMethod();
  }

  return result;
}
```

### Fallback Method 1: Password-Based Key Derivation (PBKDF2)

```javascript
/**
 * Fallback: Derive master entropy from recovery password
 * SECURITY WARNING: Lower security than PRF - use only when necessary
 */
async function deriveFromPassword(password, salt, iterations = 600000) {
  const encoder = new TextEncoder();
  
  const passwordKey = await crypto.subtle.importKey(
    'raw',
    encoder.encode(password),
    'PBKDF2',
    false,
    ['deriveKey']
  );
  
  const masterEntropy = await crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt,
      iterations, // OWASP recommends 600,000+ for PBKDF2-HMAC-SHA256
      hash: 'SHA-256'
    },
    passwordKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
  
  return masterEntropy;
}
```

### Fallback Method 2: Recovery Phrase (BIP-39)

```javascript
/**
 * Fallback: Derive master entropy from BIP-39 recovery phrase
 */
async function deriveFromRecoveryPhrase(mnemonic, passphrase = '') {
  const encoder = new TextEncoder();
  
  if (!validateMnemonic(mnemonic)) {
    throw new Error('Invalid recovery phrase');
  }
  
  const mnemonicBuffer = encoder.encode(mnemonic);
  const saltBuffer = encoder.encode('mnemonic' + passphrase);
  
  const seed = await crypto.subtle.importKey(
    'raw',
    mnemonicBuffer,
    'PBKDF2',
    false,
    ['deriveBits']
  );
  
  const seedBits = await crypto.subtle.deriveBits(
    {
      name: 'PBKDF2',
      salt: saltBuffer,
      iterations: 2048, // BIP-39 standard
      hash: 'SHA-512'
    },
    seed,
    512
  );
  
  return new Uint8Array(seedBits).slice(0, 32);
}
```

### Fallback Selection Matrix

| Scenario | PRF | Password | Recovery Phrase | Multi-Factor |
|----------|-----|----------|-----------------|--------------|
| iOS 18+ Safari | ✅ | ❌ | ⚠️ | ⚠️ |
| Android Chrome | ✅ | ❌ | ⚠️ | ⚠️ |
| Windows Hello | ❌ | ✅ | ✅ | ✅ |
| Legacy Browser | ❌ | ✅ | ✅ | ✅ |
| Lost Passkey | ❌ | ❌ | ✅ | ⚠️ |

**Legend**: ✅ Recommended, ⚠️ Possible, ❌ Not Recommended

---

## Implementation Roadmap

### Phase 1: Core Cryptography (Weeks 1-2)

- [ ] Implement HKDF-SHA256 key derivation
- [ ] Implement secp256k1 Identity Key
- [ ] Implement Iroh-compatible Transport Key
- [ ] Implement XChaCha20-Poly1305 Storage Key
- [ ] Write comprehensive tests for key derivation determinism

### Phase 2: WebAuthn PRF Integration (Weeks 3-4)

- [ ] Implement PRF support detection
- [ ] Create Passkey registration flow
- [ ] Implement PRF-based authentication
- [ ] Add fallback mechanism detection
- [ ] Test on iOS Safari, Android Chrome, Windows Hello

### Phase 3: Iroh Node Integration (Weeks 5-6)

- [ ] Set up UniFFi React Native project
- [ ] Create UDL interface definitions
- [ ] Implement Iroh node initialization
- [ ] Configure relay servers
- [ ] Test P2P connections

### Phase 4: Capability Delegation (Weeks 7-8)

- [ ] Implement UCAN token creation
- [ ] Implement UCAN validation
- [ ] Create iroh-docs namespace delegation
- [ ] Implement CRDT key prefix handlers
- [ ] Test capability revocation

### Phase 5: Mobile Background Execution (Weeks 9-10)

- [ ] iOS: Configure PushKit VoIP pushes
- [ ] iOS: Implement BGTaskScheduler
- [ ] Android: Implement Foreground Service
- [ ] Android: Set up FCM messaging
- [ ] Test "Wake-and-Sign" flow

### Phase 6: Recovery System (Weeks 11-12)

- [ ] Implement cloud vault peer discovery
- [ ] Create encrypted identity bundle format
- [ ] Implement stateless recovery flow
- [ ] Add fallback recovery methods
- [ ] Test cross-device recovery

### Phase 7: Security Audit & Optimization (Weeks 13-14)

- [ ] Security review of key derivation
- [ ] Audit UCAN delegation chains
- [ ] Test key compromise scenarios
- [ ] Optimize battery usage
- [ ] Penetration testing

---

## References

### Standards and Specifications

1. **RFC 5869** - HKDF: HMAC-based Extract-and-Expand Key Derivation Function
   - https://datatracker.ietf.org/doc/html/rfc5869

2. **WebAuthn Level 3** - PRF Extension
   - https://w3c.github.io/webauthn/#prf-extension

3. **FIDO2 CTAP 2.1** - HMAC Secret Extension
   - https://fidoalliance.org/specs/fido-v2.1-ps-20210615/

4. **UCAN Specification** - User Controlled Authorization Networks
   - https://github.com/ucan-wg/spec

5. **W3C DID Core v1.0** - Decentralized Identifiers
   - https://www.w3.org/TR/did-core/

6. **W3C Verifiable Credentials Data Model v2.0**
   - https://www.w3.org/TR/vc-data-model-2.0/

7. **NIST SP 800-63B** - Digital Identity Guidelines
   - https://pages.nist.gov/800-63-3/sp800-63b.html

8. **BIP-39** - Mnemonic Code for Generating Deterministic Keys
   - https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki

### Implementation References

9. **Iroh Documentation**
   - https://docs.iroh.computer/
   - https://docs.rs/iroh/

10. **Mozilla UniFFi**
    - https://mozilla.github.io/uniffi-rs/
    - https://jhugman.github.io/uniffi-bindgen-react-native/

11. **Yubico PRF Extension Guide**
    - https://developers.yubico.com/WebAuthn/Concepts/PRF_Extension/

---

## Document Information

- **Version**: 1.0
- **Date**: 2025
- **Classification**: Technical Implementation Roadmap
- **Scope**: Mobile-First Local-First Identity Wallet
- **Technologies**: WebAuthn PRF, Iroh, Rust, React Native, UniFFi, UCAN

---

*This document provides a comprehensive technical roadmap for implementing a stateless, local-first identity wallet using modern cryptographic primitives and peer-to-peer networking.*
