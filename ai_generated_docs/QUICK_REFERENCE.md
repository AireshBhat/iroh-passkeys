# Local-First Identity Wallet - Quick Reference Guide

## One-Page Architecture Summary

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                           LOCAL-FIRST IDENTITY WALLET                                │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│   WEBAUTHN PRF                    HKDF-SHA256                    IROH P2P           │
│   ┌──────────┐                   ┌──────────┐                   ┌──────────┐        │
│   │ Passkey  │───32-byte seed───►│ Extract  │───PRK────────────►│ Expand   │        │
│   │ (TPM/    │                   │ + Salt   │                   │          │        │
│   │  Secure  │                   │          │                   │ info="id"│        │
│   │  Enclave)│                   │          │                   │ info="tr"│        │
│   └──────────┘                   └──────────┘                   │ info="st"│        │
│                                                                  └────┬─────┘        │
│                                                                       │              │
│                              ┌────────────────────────────────────────┼──────┐       │
│                              │                                        │      │       │
│                              ▼                                        ▼      ▼       │
│                         ┌─────────┐                            ┌────────┐ ┌──────┐   │
│                         │Identity │                            │Trans-  │ │Storage│   │
│                         │Key      │                            │port Key│ │Key    │   │
│                         │(secp256k│                            │(Ed25519)│ │(XCha  │   │
│                         │1/Ed25519)│                            │        │ │Cha20) │   │
│                         └────┬────┘                            └────┬───┘ └──┬───┘   │
│                              │                                       │        │      │
│                              ▼                                       ▼        ▼      │
│                         ┌─────────┐                            ┌────────┐ ┌──────┐   │
│                         │W3C VC   │                            │Iroh    │ │iroh- │   │
│                         │Signing  │                            │Endpoint│ │docs  │   │
│                         │         │                            │(Node ID)│ │Encrypt│   │
│                         └─────────┘                            └────────┘ └──────┘   │
│                                                                                      │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

## Critical Parameters

### HKDF Info Strings

| Key Type | Info String | Algorithm |
|----------|-------------|-----------|
| Identity | `identity-wallet/v1/identity-key` | secp256k1 or Ed25519 |
| Transport | `identity-wallet/v1/transport-key` | Ed25519 (Iroh) |
| Storage | `identity-wallet/v1/storage-key` | XChaCha20-Poly1305 |

### Application Salt

```rust
const APP_SALT: &[u8] = b"LocalFirstIdentityWallet_v1_2024";
```

## Code Snippets

### 1. Derive All Keys from PRF Output

```rust
use hkdf::Hkdf;
use sha2::Sha256;

pub fn derive_keys_from_seed(master_seed: &[u8; 32]) -> Result<DerivedKeys, KeyDerivationError> {
    let hkdf = Hkdf::<Sha256>::new(Some(APP_SALT), master_seed);
    
    let mut identity_key = [0u8; 32];
    hkdf.expand(INFO_IDENTITY_KEY, &mut identity_key)?;
    
    let mut transport_key = [0u8; 32];
    hkdf.expand(INFO_TRANSPORT_KEY, &mut transport_key)?;
    
    let mut storage_key = [0u8; 32];
    hkdf.expand(INFO_STORAGE_KEY, &mut storage_key)?;
    
    Ok(DerivedKeys {
        identity_key_bytes: identity_key,
        transport_key_bytes: transport_key,
        storage_key_bytes: storage_key,
    })
}
```

### 2. Create Iroh Endpoint with Derived Key

```rust
pub async fn create_iroh_endpoint(
    transport_key_bytes: &[u8; 32]
) -> Result<iroh::Endpoint, anyhow::Error> {
    let secret_key = iroh::SecretKey::from_bytes(transport_key_bytes);
    
    let endpoint = iroh::Endpoint::builder()
        .secret_key(secret_key)
        .discovery_n0()
        .bind()
        .await?;
    
    Ok(endpoint)
}
```

### 3. WebAuthn PRF Authentication (JavaScript)

```javascript
const getOptions = {
  publicKey: {
    challenge: crypto.getRandomValues(new Uint8Array(32)),
    rpId: "wallet.example.com",
    allowCredentials: [],
    extensions: {
      prf: {
        eval: { first: credentialSalt }
      }
    }
  }
};

const assertion = await navigator.credentials.get(getOptions);
const prfResult = assertion.getClientExtensionResults()?.prf?.results?.first;
const masterSeed = new Uint8Array(prfResult);
```

### 4. UCAN Delegation Token

```rust
let ucan = UcanBuilder::default()
    .issued_by(&identity_did)
    .for_audience(&transport_did)
    .not_before(now)
    .expires_at(expiry)
    .with_capabilities(vec![
        Capability {
            with: format!("iroh:namespace:{}", namespace_id),
            can: "iroh/docs/write".to_string(),
            nb: json!({ "key_prefix": ["trust:", "claim:", "pointer:"] }),
        },
    ])
    .sign(identity_key)?;
```

### 5. iroh-docs Key Prefixes

```rust
// Trust scores
let trust_key = format!("trust:{}:professional:reliability", target_did);

// Verifiable credentials
let claim_key = format!("claim:UniversityDegree:{}:degree.name", cred_id);

// External pointers
let pointer_key = format!("pointer:service:github:username");
```

## Recovery Flow

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Passkey   │───►│    PRF      │───►│   HKDF      │───►│   Iroh      │
│   Auth      │    │   Output    │    │  Derive     │    │   Node      │
└─────────────┘    └─────────────┘    └─────────────┘    └──────┬──────┘
                                                                 │
                                                                 ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Identity  │◄───│   Decrypt   │◄───│   Vault     │◄───│   Connect   │
│   Restored  │    │   Bundle    │    │   Fetch     │    │   to Vault  │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

## Platform Support

| Platform | PRF Support | Fallback Method |
|----------|-------------|-----------------|
| iOS 18+ Safari | ✅ Native | Recovery Phrase |
| iOS 18+ Chrome | ✅ Native | Recovery Phrase |
| Android Chrome | ✅ Native | Recovery Phrase |
| macOS Safari | ✅ Native | Recovery Phrase |
| Windows Hello | ❌ No | Password + Recovery Phrase |
| Legacy Browser | ❌ No | Password + Recovery Phrase |

## Security Checklist

- [ ] Salt is application-specific and non-secret
- [ ] Info strings are unique per key type
- [ ] Master seed is zeroized after use
- [ ] UCAN tokens have short expiry (24-48 hours)
- [ ] Cloud vault uses envelope encryption
- [ ] Fallback methods show security warnings
- [ ] Rate limiting on recovery attempts
- [ ] TLS 1.3 for all network connections

## Dependencies

### Rust (Cargo.toml)

```toml
[dependencies]
hkdf = "0.12"
sha2 = "0.10"
iroh = "0.96"
ed25519-dalek = "2.1"
k256 = { version = "0.13", features = ["ecdsa", "sha256"] }
chacha20poly1305 = "0.10"
zeroize = { version = "1.8", features = ["derive"] }
ucan = "0.5"
```

### JavaScript/TypeScript

```json
{
  "dependencies": {
    "@noble/hashes": "^1.3.0",
    "@noble/curves": "^1.2.0",
    "iroh": "^0.20.0"
  }
}
```

## Common Issues

### Issue: PRF Not Supported

**Solution**: Implement fallback to PBKDF2 or recovery phrase

```javascript
const prfSupport = await detectPrfSupport();
if (!prfSupport.prfSupported) {
  return await fallbackRecovery();
}
```

### Issue: Different Node ID on Recovery

**Cause**: Different transport key bytes

**Solution**: Verify HKDF parameters match exactly

```rust
// Ensure same salt and info strings
assert_eq!(APP_SALT, b"LocalFirstIdentityWallet_v1_2024");
assert_eq!(INFO_TRANSPORT_KEY, b"identity-wallet/v1/transport-key");
```

### Issue: UCAN Validation Fails

**Cause**: Clock skew or expired token

**Solution**: Add clock skew tolerance (±60 seconds)

```rust
const CLOCK_SKEW_TOLERANCE: u64 = 60;
if timestamp.abs_diff(now) > CLOCK_SKEW_TOLERANCE {
    return Err(ValidationError::StaleProof);
}
```

## Resources

- **Full Technical Roadmap**: `TECHNICAL_ROADMAP.md`
- **HKDF Research**: `hkdf_webauthn_prf_research.md`
- **Rust Implementation**: `rust_implementation.rs`
- **Mobile Runtime**: `mobile_runtime_feasibility_report.md`
- **Capability Design**: `iroh_docs_capability_delegation_design.md`
- **Recovery Flow**: `stateless_recovery_flow_documentation.md`
