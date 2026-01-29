# Stateless Recovery Flow and Fallback Strategies
## Mobile-First Local-First Identity Wallet Documentation

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Stateless Recovery Flow Architecture](#stateless-recovery-flow-architecture)
3. [Step-by-Step Recovery Sequence](#step-by-step-recovery-sequence)
4. [Sequence Diagrams](#sequence-diagrams)
5. [Fallback Strategy for Non-PRF Devices](#fallback-strategy-for-non-prf-devices)
6. [Security Considerations](#security-considerations)
7. [Code Examples](#code-examples)
8. [References](#references)

---

## Executive Summary

This document describes the stateless recovery flow for a mobile-first Local-First identity wallet that uses WebAuthn Passkeys with PRF (Pseudo-Random Function) extension as its primary authentication mechanism. The system is designed with **NO persistent private key storage** - all cryptographic keys are deterministically derived from the PRF output during authentication.

### Key Design Principles

1. **Stateless Key Derivation**: Private keys are never stored; they are re-derived on-demand using WebAuthn PRF
2. **Cloud-Agnostic Backup**: Encrypted identity history is stored in JoFin Cloud Vault peer
3. **Device Portability**: Full identity recovery on any new device with Passkey sync
4. **Graceful Degradation**: Fallback mechanisms for devices without PRF support
5. **Zero-Knowledge Architecture**: Cloud vault cannot decrypt user data

---

## Stateless Recovery Flow Architecture

### Core Components

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

### Key Derivation Hierarchy

```
Passkey Authentication
         │
         ▼
┌─────────────────┐
│  WebAuthn PRF   │ ──► 32-byte PRF Secret (per-salt)
│  Extension      │
└─────────────────┘
         │
         ▼
┌─────────────────┐
│  HKDF Extract   │ ──► Master Key (non-extractable)
│  (SHA-256)      │
└─────────────────┘
         │
         ├──► HKDF Expand (info="identity-master-v1") ──► Master Entropy (256-bit)
         │
         ├──► HKDF Expand (info="transport-key-v1") ──► Transport Key (Ed25519)
         │
         ├──► HKDF Expand (info="encryption-key-v1") ──► Encryption Key (AES-256-GCM)
         │
         └──► HKDF Expand (info="signing-key-v1") ──► Identity Signing Key (Ed25519)
```

---

## Step-by-Step Recovery Sequence

### Phase 1: Passkey Authentication

```
┌────────────────────────────────────────────────────────────────────────────┐
│ STEP 1: User initiates recovery on new device                              │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  User Action: Tap "Recover Identity" button                                 │
│                                                                              │
│  System Action:                                                              │
│  1. Generate random challenge from server                                   │
│  2. Request WebAuthn assertion with PRF extension                           │
│  3. Prompt user for biometric/PIN authentication                            │
│                                                                              │
└────────────────────────────────────────────────────────────────────────────┘
```

### Phase 2: PRF Seed Derivation

```
┌────────────────────────────────────────────────────────────────────────────┐
│ STEP 2: Derive cryptographic seeds from Passkey                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Input:                                                                      │
│  - credentialId (from synced passkey)                                       │
│  - salt_master (stored in cloud vault metadata)                             │
│  - salt_transport (stored in cloud vault metadata)                          │
│  - salt_encryption (stored in cloud vault metadata)                         │
│                                                                              │
│  WebAuthn PRF Request:                                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ extensions: {                                                        │   │
│  │   prf: {                                                             │   │
│  │     eval: {                                                          │   │
│  │       first:  salt_master,    // 32 bytes                            │   │
│  │       second: salt_transport, // 32 bytes                            │   │
│  │       // additional salts can be requested                           │   │
│  │     }                                                                │   │
│  │   }                                                                  │   │
│  │ }                                                                    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  Output: 32-byte PRF results per salt (hardware-bound, deterministic)       │
│                                                                              │
└────────────────────────────────────────────────────────────────────────────┘
```

### Phase 3: Master Entropy Re-derivation

```
┌────────────────────────────────────────────────────────────────────────────┐
│ STEP 3: Re-derive Master Entropy using HKDF                                │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ // Import PRF result as HKDF key material                            │   │
│  │ const masterKey = await crypto.subtle.importKey(                     │   │
│  │   'raw',                                                             │   │
│  │   prfResult,                                                         │   │
│  │   'HKDF',                                                            │   │
│  │   false,           // non-extractable                                │   │
│  │   ['deriveKey']                                                      │   │
│  │ );                                                                   │   │
│  │                                                                      │   │
│  │ // Derive Master Entropy                                             │   │
│  │ const masterEntropy = await crypto.subtle.deriveKey(                 │   │
│  │   {                                                                  │   │
│  │     name: 'HKDF',                                                    │   │
│  │     salt: new Uint8Array(),  // empty salt (PRF is already strong)   │   │
│  │     hash: 'SHA-256',                                                 │   │
│  │     info: new TextEncoder().encode('jofin-identity-master-v1')       │   │
│  │   },                                                                 │   │
│  │   masterKey,                                                         │   │
│  │   { name: 'AES-GCM', length: 256 },                                  │   │
│  │   false,                                                             │   │
│  │   ['encrypt', 'decrypt']                                             │   │
│  │ );                                                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  Result: Identical Master Entropy to original device (deterministic)        │
│                                                                              │
└────────────────────────────────────────────────────────────────────────────┘
```

### Phase 4: Transport Key Regeneration

```
┌────────────────────────────────────────────────────────────────────────────┐
│ STEP 4: Regenerate Transport Key for Iroh Network                          │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  The Transport Key is an Ed25519 key pair used for:                         │
│  - Iroh endpoint identification                                             │
│  - QUIC connection authentication                                           │
│  - Peer-to-peer protocol handshakes                                         │
│                                                                              │
│  Derivation Path:                                                            │
│  PRF(salt_transport) ──► HKDF ──► Ed25519 Seed ──► Key Pair                │
│                                                                              │
│  Properties:                                                                 │
│  - Same Transport Key = Same Endpoint ID on new device                      │
│  - Existing peers can recognize the identity                                │
│  - No need to re-establish trust relationships                              │
│                                                                              │
└────────────────────────────────────────────────────────────────────────────┘
```

### Phase 5: Iroh Node Initialization

```
┌────────────────────────────────────────────────────────────────────────────┐
│ STEP 5: Initialize Iroh Node with Regenerated Keys                         │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Configuration:                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ const endpoint = await iroh.Endpoint.create({                        │   │
│  │   // Use regenerated transport key                                   │   │
│  │   privateKey: transportKeyPair.privateKey,                           │   │
│  │   publicKey: transportKeyPair.publicKey,                             │   │
│  │                                                                      │   │
│  │   // Enable discovery services                                       │   │
│  │   discovery: [                                                       │   │
│  │     iroh.discovery.DnsDiscovery.default(),                           │   │
│  │     iroh.discovery.PkarrDiscovery.default(),                         │   │
│  │     iroh.discovery.DhtDiscovery.default()  // optional               │   │
│  │   ],                                                                 │   │
│  │                                                                      │   │
│  │   // Configure relays                                                │   │
│  │   relayMode: iroh.RelayMode.enabled(),                               │   │
│  │                                                                      │   │
│  │   // Enable hole punching                                            │   │
│  │   holePunching: true                                                 │   │
│  │ });                                                                  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  Result: Iroh node starts with same Endpoint ID as original device          │
│                                                                              │
└────────────────────────────────────────────────────────────────────────────┘
```

### Phase 6: Cloud Vault Peer Discovery

```
┌────────────────────────────────────────────────────────────────────────────┐
│ STEP 6: Discover JoFin Cloud Vault Peer                                    │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Discovery Methods (in order of preference):                                │
│                                                                              │
│  1. DNS Discovery (default):                                                │
│     - Query: _iroh.<z32-vault-endpoint-id>.dns.iroh.link                   │
│     - Returns: TXT record with relay URL and addressing info               │
│                                                                              │
│  2. Pkarr Discovery:                                                        │
│     - Resolve vault EndpointID via pkarr servers                           │
│     - Returns: Signed DNS packet with relay URL                            │
│                                                                              │
│  3. DHT Discovery (fallback):                                               │
│     - Query BitTorrent Mainline DHT for vault endpoint                     │
│     - Returns: Relay URL and direct addresses                              │
│                                                                              │
│  4. Bootstrap Nodes (last resort):                                          │
│     - Hardcoded list of known vault peer addresses                         │
│     - Used when other discovery methods fail                               │
│                                                                              │
└────────────────────────────────────────────────────────────────────────────┘
```

### Phase 7: Encrypted Data Synchronization

```
┌────────────────────────────────────────────────────────────────────────────┐
│ STEP 7: Fetch and Decrypt Identity History from Cloud Vault                │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ // 1. Connect to vault peer using discovered addressing info         │   │
│  │ const vaultConnection = await endpoint.connect(                      │   │
│  │   vaultEndpointId,                                                   │   │
│  │   vaultRelayUrl                                                      │   │
│  │ );                                                                   │   │
│  │                                                                      │   │
│  │ // 2. Request encrypted identity bundle                              │   │
│  │ const encryptedBundle = await vaultConnection.request(               │   │
│  │   'identity.sync',                                                   │   │
│  │   { identityHash: deriveIdentityHash(masterEntropy) }                │   │
│  │ );                                                                   │   │
│  │                                                                      │   │
│  │ // 3. Decrypt identity bundle using encryption key                   │   │
│  │ const encryptionKey = await deriveEncryptionKey(masterEntropy);      │   │
│  │ const identityBundle = await decryptBundle(                          │   │
│  │   encryptedBundle,                                                   │   │
│  │   encryptionKey                                                      │   │
│  │ );                                                                   │   │
│  │                                                                      │   │
│  │ // 4. Verify bundle integrity and authenticity                       │   │
│  │ const isValid = await verifyBundleSignature(                         │   │
│  │   identityBundle,                                                    │   │
│  │   identityBundle.signingKey                                          │   │
│  │ );                                                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  Bundle Contents:                                                            │
│  - Identity metadata (name, avatar, etc.)                                   │
│  - Credential list (DIDs, linked accounts)                                  │
│  - Contact list (encrypted)                                                 │
│  - Transaction history (encrypted)                                          │
│  - Device authorization records                                             │
│  - Salt values for future key derivation                                    │
│                                                                              │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Sequence Diagrams

### Full Recovery Flow Sequence

```
┌─────────┐     ┌──────────────┐     ┌─────────────┐     ┌────────────┐     ┌─────────────┐     ┌──────────────┐
│  User   │     │   New Device │     │  Passkey    │     │   Iroh     │     │   Cloud     │     │  JoFin Vault │
│         │     │  (Wallet)    │     │  Provider   │     │   Node     │     │  Discovery  │     │   Peer       │
└────┬────┘     └──────┬───────┘     └──────┬──────┘     └─────┬──────┘     └──────┬──────┘     └──────┬───────┘
     │                 │                    │                  │                   │                   │
     │ 1. Init Recovery │                    │                  │                   │                   │
     │────────────────>│                    │                  │                   │                   │
     │                 │                    │                  │                   │                   │
     │                 │ 2. Request Auth    │                  │                   │                   │
     │                 │───────────────────>│                  │                   │                   │
     │                 │                    │                  │                   │                   │
     │                 │                    │ 3. Biometric/PIN │                   │                   │
     │                 │                    │◄─────────────────│                   │                   │
     │                 │                    │                  │                   │                   │
     │                 │ 4. Assertion + PRF │                  │                   │                   │
     │                 │◄───────────────────│                  │                   │                   │
     │                 │                    │                  │                   │                   │
     │                 │ 5. Derive Keys     │                  │                   │                   │
     │                 │─────────────────────────────────────────>│                   │                   │
     │                 │                    │                  │                   │                   │
     │                 │ 6. Init Iroh Node  │                  │                   │                   │
     │                 │─────────────────────────────────────────>│                   │                   │
     │                 │                    │                  │                   │                   │
     │                 │ 7. Query Discovery │                  │                   │                   │
     │                 │──────────────────────────────────────────────────────────>│                   │
     │                 │                    │                  │                   │                   │
     │                 │ 8. Vault Peer Info │                  │                   │                   │
     │                 │◄──────────────────────────────────────────────────────────│                   │
     │                 │                    │                  │                   │                   │
     │                 │ 9. Connect to Vault│                  │                   │                   │
     │                 │──────────────────────────────────────────────────────────────────────────────>│
     │                 │                    │                  │                   │                   │
     │                 │ 10. Request Sync   │                  │                   │                   │
     │                 │──────────────────────────────────────────────────────────────────────────────>│
     │                 │                    │                  │                   │                   │
     │                 │ 11. Encrypted Data │                  │                   │                   │
     │                 │◄──────────────────────────────────────────────────────────────────────────────│
     │                 │                    │                  │                   │                   │
     │                 │ 12. Decrypt & Verify│                 │                   │                   │
     │                 │─────────────────────────────────────────>│                   │                   │
     │                 │                    │                  │                   │                   │
     │ 13. Recovery Complete               │                  │                   │                   │
     │◄────────────────────────────────────│                  │                   │                   │
     │                 │                    │                  │                   │                   │
```

### Key Derivation Sequence

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                           KEY DERIVATION SEQUENCE                                        │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│   Passkey Auth                                                                           │
│        │                                                                                 │
│        ▼                                                                                 │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐           │
│   │  WebAuthn   │────►│  PRF Result │────►│  HKDF       │────►│ Master Key  │           │
│   │  get()      │     │  (32 bytes) │     │  Extract    │     │ (non-extract)│          │
│   └─────────────┘     └─────────────┘     └─────────────┘     └──────┬──────┘           │
│                                                                      │                    │
│                              ┌───────────────────────────────────────┼────────┐          │
│                              │                                       │        │          │
│                              ▼                                       ▼        ▼          │
│                         ┌─────────┐                            ┌─────────┐ ┌─────────┐    │
│                         │ HKDF    │                            │ HKDF    │ │ HKDF    │    │
│                         │ Expand  │                            │ Expand  │ │ Expand  │    │
│                         │ (info=  │                            │ (info=  │ │ (info=  │    │
│                         │ "master"│                            │ "trans" │ │ "enc" ) │    │
│                         └────┬────┘                            └────┬────┘ └────┬────┘    │
│                              │                                       │         │         │
│                              ▼                                       ▼         ▼         │
│                         ┌─────────┐                            ┌─────────┐ ┌─────────┐    │
│                         │ Master  │                            │Transport│ │Encrypt  │    │
│                         │ Entropy │                            │ Key     │ │ Key     │    │
│                         │ (256b)  │                            │(Ed25519)│ │(AES256) │    │
│                         └────┬────┘                            └────┬────┘ └────┬────┘    │
│                              │                                       │         │         │
│                              ▼                                       ▼         ▼         │
│                         ┌─────────┐                            ┌─────────┐ ┌─────────┐    │
│                         │ Identity│                            │ Iroh    │ │ Decrypt │    │
│                         │ Signing │                            │ Endpoint│ │ Vault   │    │
│                         │ Key     │                            │ ID      │ │ Data    │    │
│                         └─────────┘                            └─────────┘ └─────────┘    │
│                                                                                          │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Fallback Strategy for Non-PRF Devices

### PRF Support Detection

```javascript
/**
 * Detect PRF extension support before attempting recovery
 * @returns {Promise<Object>} Support status and fallback recommendation
 */
async function detectPrfSupport() {
  const result = {
    prfSupported: false,
    fallbackRequired: false,
    fallbackMethod: null,
    browserInfo: {
      userAgent: navigator.userAgent,
      platform: navigator.platform
    }
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
    
    // Note: This will fail without user gesture, but we can check extension handling
    const credential = await navigator.credentials.create(createOptions);
    const extResults = credential.getClientExtensionResults();
    result.prfSupported = extResults.prf?.enabled === true;
  } catch (e) {
    // Expected to fail - we only care about extension handling
  }

  // Determine fallback method
  if (!result.prfSupported) {
    result.fallbackRequired = true;
    result.fallbackMethod = selectFallbackMethod();
  }

  return result;
}
```

### Fallback Method 1: Password-Based Key Derivation (PBKDF2)

**When to use**: Device has no PRF support, user has recovery password

**Security Trade-offs**:
- ❌ Lower entropy source (user-chosen password)
- ❌ Vulnerable to brute-force attacks
- ❌ Not hardware-backed
- ✅ Widely supported
- ✅ User familiar with password entry

```javascript
/**
 * Fallback: Derive master entropy from recovery password
 * SECURITY WARNING: Lower security than PRF - use only when necessary
 */
async function deriveFromPassword(password, salt, iterations = 600000) {
  const encoder = new TextEncoder();
  
  // Import password as key material
  const passwordKey = await crypto.subtle.importKey(
    'raw',
    encoder.encode(password),
    'PBKDF2',
    false,
    ['deriveKey']
  );
  
  // Derive master entropy using PBKDF2
  // NIST SP 800-132 recommends minimum 1000 iterations
  // OWASP recommends 600,000+ for PBKDF2-HMAC-SHA256
  const masterEntropy = await crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: salt,
      iterations: iterations,
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

### Fallback Method 2: Recovery Phrase (BIP-39 Style)

**When to use**: Device has no PRF support, user has recovery phrase

**Security Trade-offs**:
- ❌ Requires secure storage of recovery phrase
- ❌ Phrase can be stolen if written down
- ✅ High entropy (128-256 bits)
- ✅ Works offline
- ✅ Industry standard (BIP-39)

```javascript
/**
 * Fallback: Derive master entropy from BIP-39 recovery phrase
 * Uses PBKDF2 with "mnemonic" salt as per BIP-39 spec
 */
async function deriveFromRecoveryPhrase(mnemonic, passphrase = '') {
  const encoder = new TextEncoder();
  
  // Validate mnemonic (implementation omitted for brevity)
  if (!validateMnemonic(mnemonic)) {
    throw new Error('Invalid recovery phrase');
  }
  
  // Convert mnemonic to seed using BIP-39 standard
  // PBKDF2 with HMAC-SHA512, 2048 iterations
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
      iterations: 2048,
      hash: 'SHA-512'
    },
    seed,
    512  // 64 bytes
  );
  
  // Use first 32 bytes as master entropy
  const masterEntropy = seedBits.slice(0, 32);
  
  return masterEntropy;
}
```

### Fallback Method 3: Multi-Factor Recovery

**When to use**: Device has no PRF support, multiple recovery factors available

```javascript
/**
 * Fallback: Combine multiple weak factors into stronger key
 * Uses Shamir's Secret Sharing or XOR combination
 */
async function deriveFromMultipleFactors(factors, method = 'xor') {
  // factors: Array of { type: 'password'|'pin'|'answer', value: string }
  
  if (method === 'xor') {
    // XOR combination (all factors required)
    let combined = null;
    
    for (const factor of factors) {
      const factorBytes = await hashFactor(factor);
      if (combined === null) {
        combined = new Uint8Array(factorBytes);
      } else {
        for (let i = 0; i < combined.length; i++) {
          combined[i] ^= factorBytes[i];
        }
      }
    }
    
    return deriveFromBytes(combined);
  } 
  else if (method === 'shamir') {
    // Shamir's Secret Sharing (M of N factors required)
    // Requires pre-generated shares during setup
    throw new Error('Shamir recovery not implemented');
  }
}
```

### Fallback Selection Matrix

| Scenario | PRF | Password | Recovery Phrase | Multi-Factor |
|----------|-----|----------|-----------------|--------------|
| iOS 18+ Safari | ✅ | ❌ | ⚠️ | ⚠️ |
| Android Chrome | ✅ | ❌ | ⚠️ | ⚠️ |
| Windows Hello | ❌ | ✅ | ✅ | ✅ |
| Legacy Browser | ❌ | ✅ | ✅ | ✅ |
| Hardware Key Only | ⚠️ | ❌ | ✅ | ❌ |
| Lost Passkey | ❌ | ❌ | ✅ | ⚠️ |

**Legend**: ✅ Recommended, ⚠️ Possible, ❌ Not Recommended

---

## Security Considerations

### Cloud Vault Encryption Requirements

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ENCRYPTION ARCHITECTURE                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    ENVELOPE ENCRYPTION PATTERN                       │    │
│  │                                                                      │    │
│  │   ┌──────────────┐         ┌──────────────┐         ┌─────────────┐ │    │
│  │   │   Identity   │         │    Data      │         │   Wrapped   │ │    │
│  │   │    Data      │◄────────│ Encryption   │◄────────│    DEK      │ │    │
│  │   │   (Vault)    │  DEK    │   Key (DEK)  │  KEK    │  (by PRF)   │ │    │
│  │   └──────────────┘         └──────────────┘         └─────────────┘ │    │
│  │                                                              │       │    │
│  │                                                              │       │    │
│  │   Cloud Storage:                                             ▼       │    │
│  │   ┌─────────────────────────────────────────────────────────────────┐│    │
│  │   │  Encrypted Identity Bundle + Wrapped DEK + Metadata            ││    │
│  │   │                                                                ││    │
│  │   │  - AES-256-GCM encrypted data                                  ││    │
│  │   │  - 96-bit nonce per encryption                                 ││    │
│  │   │  - Authentication tag (128-bit)                                ││    │
│  │   │  - Wrapped DEK (encrypted with PRF-derived KEK)                ││    │
│  │   └─────────────────────────────────────────────────────────────────┘│    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  Security Properties:                                                        │
│  ✓ Cloud provider cannot decrypt data (zero-knowledge)                      │
│  ✓ Each device has unique KEK (via unique PRF salt)                         │
│  ✓ Compromised device key doesn't affect other devices                      │
│  ✓ Forward secrecy via periodic salt rotation                               │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Peer Authentication and Verification

```javascript
/**
 * Verify cloud vault peer authenticity
 * Prevents man-in-the-middle attacks during recovery
 */
async function verifyVaultPeer(connection, expectedVaultId) {
  // 1. Verify endpoint ID matches expected vault
  const remoteEndpointId = connection.remoteEndpointId;
  if (remoteEndpointId !== expectedVaultId) {
    throw new Error('Vault peer ID mismatch - possible MITM attack');
  }
  
  // 2. Verify TLS certificate (handled by QUIC)
  const tlsInfo = await connection.getTlsInfo();
  if (!tlsInfo.verified) {
    throw new Error('TLS verification failed');
  }
  
  // 3. Challenge-response authentication
  const challenge = crypto.getRandomValues(new Uint8Array(32));
  const response = await connection.request('auth.challenge', { challenge });
  
  // Verify response signed by vault's identity key
  const isValid = await verifySignature(
    challenge,
    response.signature,
    expectedVaultId
  );
  
  if (!isValid) {
    throw new Error('Vault authentication failed - invalid signature');
  }
  
  return true;
}
```

### Anti-Phishing Measures

```javascript
/**
 * Anti-phishing verification during recovery
 */
async function verifyRecoveryContext() {
  // 1. Verify relying party ID matches expected domain
  const expectedRpId = 'wallet.jofin.io';
  const currentOrigin = new URL(window.location.origin).hostname;
  
  if (!currentOrigin.endsWith(expectedRpId)) {
    throw new Error('Phishing detected: Invalid relying party');
  }
  
  // 2. Verify certificate transparency (for hosted wallets)
  // Implementation depends on deployment model
  
  // 3. Check for expected UI elements (visual verification)
  const expectedElements = [
    'jofin-recovery-header',
    'jofin-security-indicator'
  ];
  
  for (const elementId of expectedElements) {
    if (!document.getElementById(elementId)) {
      console.warn('Unexpected UI - possible phishing attempt');
    }
  }
  
  // 4. Rate limiting for recovery attempts
  const recoveryAttempts = await getRecoveryAttemptCount();
  if (recoveryAttempts > 5) {
    throw new Error('Too many recovery attempts - please wait');
  }
  
  return true;
}
```

### NIST SP 800-63B Compliance

| Requirement | Implementation |
|-------------|----------------|
| AAL2 Minimum | Passkey (possession) + Biometric/PIN (inherence/knowledge) |
| Authenticator Binding | PRF-derived keys bound to passkey credential |
| Account Recovery | Recovery phrase + multi-device passkey sync |
| Rate Limiting | 5 attempts per hour, exponential backoff |
| Session Management | 15-minute inactivity timeout, 12-hour max session |
| Notification | Push notification to all linked devices on recovery |

---

## Code Examples

### Complete Recovery Flow Implementation

```typescript
// recovery.ts - Complete stateless recovery implementation

import { iroh } from 'iroh';
import { hkdf } from '@noble/hashes/hkdf';
import { sha256 } from '@noble/hashes/sha256';
import { ed25519 } from '@noble/curves/ed25519';

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

/**
 * Phase 1: WebAuthn authentication with PRF extension
 */
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
          // Can request up to 2 salts per authentication
        }
      }
    }
  };
  
  const assertion = await navigator.credentials.get({ publicKey: getOptions });
  const extResults = assertion.getClientExtensionResults();
  
  if (!extResults.prf?.results?.first) {
    throw new Error('PRF extension failed to return results');
  }
  
  return {
    master: new Uint8Array(extResults.prf.results.first),
    transport: new Uint8Array(extResults.prf.results.second),
    credentialId: assertion.id
  };
}

/**
 * Phase 2: Derive all cryptographic keys from PRF results
 */
async function deriveAllKeys(prfResults: PrfResults): Promise<DerivedKeys> {
  // Derive master entropy
  const masterEntropy = await deriveKeyFromPrf(
    prfResults.master,
    'jofin-identity-master-v1'
  );
  
  // Derive transport key (Ed25519)
  const transportSeed = await deriveBytesFromPrf(
    prfResults.transport,
    'jofin-transport-key-v1',
    32
  );
  const transportKeyPair = ed25519.utils.randomPrivateKey(); // Use as seed
  
  // Derive encryption key (AES-256)
  const encryptionKey = await deriveKeyFromPrf(
    prfResults.master,
    'jofin-encryption-key-v1'
  );
  
  // Derive signing key (Ed25519)
  const signingSeed = await deriveBytesFromPrf(
    prfResults.master,
    'jofin-signing-key-v1',
    32
  );
  const signingKeyPair = ed25519.utils.randomPrivateKey();
  
  return {
    masterEntropy,
    transportKeyPair: {
      privateKey: transportSeed,
      publicKey: ed25519.getPublicKey(transportSeed)
    },
    encryptionKey,
    signingKeyPair: {
      privateKey: signingSeed,
      publicKey: ed25519.getPublicKey(signingSeed)
    }
  };
}

/**
 * Phase 3: Initialize Iroh node with regenerated keys
 */
async function initializeIrohNode(
  transportKeyPair: KeyPair
): Promise<iroh.Endpoint> {
  const endpoint = await iroh.Endpoint.create({
    // Use regenerated key pair for consistent Endpoint ID
    secretKey: transportKeyPair.privateKey,
    
    // Enable all discovery methods
    discovery: [
      iroh.discovery.DnsDiscovery.default(),
      iroh.discovery.PkarrDiscovery.default(),
    ],
    
    // Enable relay for NAT traversal
    relayMode: iroh.RelayMode.enabled(),
    
    // Enable hole punching
    holePunching: true,
    
    // Custom protocols
    protocols: [
      new IdentitySyncProtocol(),
      new VaultSyncProtocol()
    ]
  });
  
  return endpoint;
}

/**
 * Phase 4: Connect to cloud vault peer
 */
async function connectToVault(
  endpoint: iroh.Endpoint,
  config: RecoveryConfig
): Promise<iroh.Connection> {
  // Discover vault peer addressing info
  let vaultAddr = await discoverVaultPeer(config);
  
  if (!vaultAddr && config.vaultRelayUrl) {
    vaultAddr = {
      endpointId: config.vaultEndpointId,
      relayUrl: config.vaultRelayUrl
    };
  }
  
  if (!vaultAddr) {
    throw new Error('Could not discover vault peer');
  }
  
  // Connect to vault
  const connection = await endpoint.connect(
    vaultAddr.endpointId,
    vaultAddr.relayUrl
  );
  
  // Verify vault authenticity
  await verifyVaultPeer(connection, config.vaultEndpointId);
  
  return connection;
}

/**
 * Phase 5: Fetch and decrypt identity bundle
 */
async function fetchAndDecryptIdentity(
  connection: iroh.Connection,
  encryptionKey: CryptoKey
): Promise<IdentityBundle> {
  // Request encrypted bundle from vault
  const response = await connection.request('vault.getIdentity', {
    timestamp: Date.now()
  });
  
  if (!response.encryptedBundle) {
    throw new Error('No identity bundle found in vault');
  }
  
  // Decrypt bundle
  const encryptedData = base64ToArrayBuffer(response.encryptedBundle);
  const iv = encryptedData.slice(0, 12);
  const ciphertext = encryptedData.slice(12);
  
  const decrypted = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv },
    encryptionKey,
    ciphertext
  );
  
  const identityBundle: IdentityBundle = JSON.parse(
    new TextDecoder().decode(decrypted)
  );
  
  return identityBundle;
}

/**
 * Phase 6: Verify identity bundle integrity
 */
async function verifyIdentityBundle(
  bundle: IdentityBundle,
  signingKey: KeyPair
): Promise<void> {
  // Verify bundle signature
  const dataToVerify = JSON.stringify(bundle.data);
  const isValid = await verifyEd25519Signature(
    dataToVerify,
    bundle.signature,
    bundle.signingKey
  );
  
  if (!isValid) {
    throw new Error('Identity bundle signature verification failed');
  }
  
  // Verify signing key matches derived key
  if (!constantTimeEqual(signingKey.publicKey, bundle.signingKey)) {
    throw new Error('Signing key mismatch - possible tampering');
  }
}

// Helper functions

async function deriveKeyFromPrf(
  prfResult: Uint8Array,
  info: string
): Promise<CryptoKey> {
  const masterKey = await crypto.subtle.importKey(
    'raw',
    prfResult,
    'HKDF',
    false,
    ['deriveKey']
  );
  
  return crypto.subtle.deriveKey(
    {
      name: 'HKDF',
      salt: new Uint8Array(),
      hash: 'SHA-256',
      info: new TextEncoder().encode(info)
    },
    masterKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
}

async function deriveBytesFromPrf(
  prfResult: Uint8Array,
  info: string,
  length: number
): Promise<Uint8Array> {
  const masterKey = await crypto.subtle.importKey(
    'raw',
    prfResult,
    'HKDF',
    false,
    ['deriveBits']
  );
  
  const bits = await crypto.subtle.deriveBits(
    {
      name: 'HKDF',
      salt: new Uint8Array(),
      hash: 'SHA-256',
      info: new TextEncoder().encode(info)
    },
    masterKey,
    length * 8
  );
  
  return new Uint8Array(bits);
}
```

### Cloud Vault Peer Connection

```typescript
// vault-connection.ts - Cloud vault peer protocol implementation

import { iroh } from 'iroh';

/**
 * Custom protocol for vault synchronization
 */
export class VaultSyncProtocol implements iroh.Protocol {
  readonly id = '/jofin/vault-sync/1.0.0';
  
  async handleConnection(conn: iroh.Connection): Promise<void> {
    // Handle incoming vault sync requests
    const stream = await conn.acceptStream();
    const request = await stream.readJson();
    
    switch (request.method) {
      case 'vault.getIdentity':
        await this.handleGetIdentity(stream, request);
        break;
      case 'vault.putIdentity':
        await this.handlePutIdentity(stream, request);
        break;
      case 'vault.syncHistory':
        await this.handleSyncHistory(stream, request);
        break;
      default:
        await stream.writeJson({ error: 'Unknown method' });
    }
  }
  
  private async handleGetIdentity(
    stream: iroh.Stream,
    request: any
  ): Promise<void> {
    // Authenticate request
    const identityHash = request.params.identityHash;
    
    // Retrieve encrypted bundle from storage
    const bundle = await this.storage.getIdentityBundle(identityHash);
    
    if (!bundle) {
      await stream.writeJson({ error: 'Identity not found' });
      return;
    }
    
    // Return encrypted bundle (client will decrypt)
    await stream.writeJson({
      encryptedBundle: arrayBufferToBase64(bundle.data),
      metadata: bundle.metadata,
      timestamp: bundle.timestamp
    });
  }
}

/**
 * Discover vault peer using multiple methods
 */
export async function discoverVaultPeer(
  config: RecoveryConfig
): Promise<VaultAddress | null> {
  // Try DNS discovery first
  if (config.discoveryServices.includes('dns')) {
    try {
      const dnsResult = await dnsDiscovery(config.vaultEndpointId);
      if (dnsResult) return dnsResult;
    } catch (e) {
      console.warn('DNS discovery failed:', e);
    }
  }
  
  // Try Pkarr discovery
  if (config.discoveryServices.includes('pkarr')) {
    try {
      const pkarrResult = await pkarrDiscovery(config.vaultEndpointId);
      if (pkarrResult) return pkarrResult;
    } catch (e) {
      console.warn('Pkarr discovery failed:', e);
    }
  }
  
  // Try DHT discovery as last resort
  if (config.discoveryServices.includes('dht')) {
    try {
      const dhtResult = await dhtDiscovery(config.vaultEndpointId);
      if (dhtResult) return dhtResult;
    } catch (e) {
      console.warn('DHT discovery failed:', e);
    }
  }
  
  return null;
}

/**
 * DNS-based vault discovery
 */
async function dnsDiscovery(endpointId: string): Promise<VaultAddress | null> {
  // Convert endpoint ID to z32 encoding
  const z32Id = toZ32Encoding(endpointId);
  
  // Query DNS TXT record
  const query = `_iroh.${z32Id}.dns.iroh.link`;
  
  try {
    const response = await dns.resolveTxt(query);
    
    // Parse TXT record for relay URL and addresses
    const records = response.Answer.map(r => r.data);
    const relayUrl = records.find(r => r.startsWith('relay='))?.split('=')[1];
    const addrs = records.find(r => r.startsWith('addrs='))?.split('=')[1]?.split(',');
    
    if (relayUrl) {
      return {
        endpointId,
        relayUrl,
        directAddrs: addrs
      };
    }
  } catch (e) {
    console.warn('DNS query failed:', e);
  }
  
  return null;
}

/**
 * Pkarr-based vault discovery
 */
async function pkarrDiscovery(endpointId: string): Promise<VaultAddress | null> {
  const pkarr = new Pkarr();
  
  try {
    const packet = await pkarr.resolve(endpointId);
    
    // Extract relay URL from packet
    const relayRecord = packet.answers.find(
      r => r.name === '_iroh' && r.type === 'TXT'
    );
    
    if (relayRecord) {
      const relayUrl = relayRecord.data;
      return {
        endpointId,
        relayUrl
      };
    }
  } catch (e) {
    console.warn('Pkarr resolution failed:', e);
  }
  
  return null;
}
```

### Fallback Mechanism Detection and Handling

```typescript
// fallback-handler.ts - PRF fallback implementation

interface FallbackStrategy {
  type: 'password' | 'recovery-phrase' | 'multi-factor';
  execute: () => Promise<Uint8Array>;
  securityLevel: 'high' | 'medium' | 'low';
}

/**
 * Main fallback handler - selects and executes appropriate recovery method
 */
export class FallbackRecoveryHandler {
  private strategies: Map<string, FallbackStrategy> = new Map();
  
  constructor() {
    this.registerDefaultStrategies();
  }
  
  private registerDefaultStrategies(): void {
    // Password-based recovery
    this.strategies.set('password', {
      type: 'password',
      securityLevel: 'medium',
      execute: async () => {
        const password = await this.promptForPassword();
        const salt = await this.getRecoverySalt();
        return this.deriveFromPassword(password, salt);
      }
    });
    
    // Recovery phrase (BIP-39)
    this.strategies.set('recovery-phrase', {
      type: 'recovery-phrase',
      securityLevel: 'high',
      execute: async () => {
        const phrase = await this.promptForRecoveryPhrase();
        const passphrase = await this.promptForPassphrase(); // Optional
        return this.deriveFromRecoveryPhrase(phrase, passphrase);
      }
    });
    
    // Multi-factor recovery
    this.strategies.set('multi-factor', {
      type: 'multi-factor',
      securityLevel: 'high',
      execute: async () => {
        const factors = await this.collectFactors();
        return this.deriveFromMultipleFactors(factors);
      }
    });
  }
  
  /**
   * Attempt recovery using fallback methods
   */
  async attemptFallbackRecovery(): Promise<RecoveryResult> {
    // Show user available fallback options
    const availableStrategies = this.getAvailableStrategies();
    
    const selectedStrategy = await this.promptUserForStrategy(availableStrategies);
    
    if (!selectedStrategy) {
      throw new Error('No recovery method selected');
    }
    
    // Show security warning for lower-security methods
    if (selectedStrategy.securityLevel !== 'high') {
      const confirmed = await this.showSecurityWarning(selectedStrategy);
      if (!confirmed) {
        throw new Error('User declined lower-security recovery');
      }
    }
    
    // Execute selected strategy
    try {
      const masterEntropy = await selectedStrategy.execute();
      
      // Verify derived entropy matches expected identity
      const identityHash = await this.deriveIdentityHash(masterEntropy);
      const isValid = await this.verifyIdentityHash(identityHash);
      
      if (!isValid) {
        throw new Error('Derived identity does not match - incorrect recovery data');
      }
      
      return {
        success: true,
        masterEntropy,
        method: selectedStrategy.type
      };
      
    } catch (error) {
      return {
        success: false,
        error: error.message,
        method: selectedStrategy.type
      };
    }
  }
  
  /**
   * Derive master entropy from password (PBKDF2)
   */
  private async deriveFromPassword(
    password: string,
    salt: Uint8Array
  ): Promise<Uint8Array> {
    const encoder = new TextEncoder();
    
    const keyMaterial = await crypto.subtle.importKey(
      'raw',
      encoder.encode(password),
      'PBKDF2',
      false,
      ['deriveBits']
    );
    
    const bits = await crypto.subtle.deriveBits(
      {
        name: 'PBKDF2',
        salt,
        iterations: 600000, // OWASP recommendation
        hash: 'SHA-256'
      },
      keyMaterial,
      256
    );
    
    return new Uint8Array(bits);
  }
  
  /**
   * Derive master entropy from BIP-39 recovery phrase
   */
  private async deriveFromRecoveryPhrase(
    mnemonic: string,
    passphrase: string = ''
  ): Promise<Uint8Array> {
    // Validate mnemonic
    if (!validateMnemonic(mnemonic)) {
      throw new Error('Invalid recovery phrase');
    }
    
    const encoder = new TextEncoder();
    const mnemonicBuffer = encoder.encode(mnemonic);
    const saltBuffer = encoder.encode('mnemonic' + passphrase);
    
    const keyMaterial = await crypto.subtle.importKey(
      'raw',
      mnemonicBuffer,
      'PBKDF2',
      false,
      ['deriveBits']
    );
    
    const bits = await crypto.subtle.deriveBits(
      {
        name: 'PBKDF2',
        salt: saltBuffer,
        iterations: 2048, // BIP-39 standard
        hash: 'SHA-512'
      },
      keyMaterial,
      512
    );
    
    // Return first 32 bytes as master entropy
    return new Uint8Array(bits).slice(0, 32);
  }
  
  /**
   * Derive from multiple factors using XOR combination
   */
  private async deriveFromMultipleFactors(
    factors: Array<{ type: string; value: string }>
  ): Promise<Uint8Array> {
    if (factors.length < 2) {
      throw new Error('At least 2 factors required for multi-factor recovery');
    }
    
    let combined: Uint8Array | null = null;
    
    for (const factor of factors) {
      // Hash each factor to fixed length
      const factorBytes = await this.hashFactor(factor);
      
      if (combined === null) {
        combined = factorBytes;
      } else {
        // XOR combination
        for (let i = 0; i < combined.length; i++) {
          combined[i] ^= factorBytes[i];
        }
      }
    }
    
    return combined!;
  }
  
  private async hashFactor(factor: { type: string; value: string }): Promise<Uint8Array> {
    const encoder = new TextEncoder();
    const data = encoder.encode(`${factor.type}:${factor.value}`);
    
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    return new Uint8Array(hashBuffer);
  }
  
  // UI prompt methods (implementations depend on platform)
  private async promptForPassword(): Promise<string> { /* ... */ }
  private async promptForRecoveryPhrase(): Promise<string> { /* ... */ }
  private async promptForPassphrase(): Promise<string> { /* ... */ }
  private async promptUserForStrategy(strategies: FallbackStrategy[]): Promise<FallbackStrategy | null> { /* ... */ }
  private async showSecurityWarning(strategy: FallbackStrategy): Promise<boolean> { /* ... */ }
  private getAvailableStrategies(): FallbackStrategy[] { /* ... */ }
  private async getRecoverySalt(): Promise<Uint8Array> { /* ... */ }
  private async collectFactors(): Promise<Array<{ type: string; value: string }>> { /* ... */ }
  private async deriveIdentityHash(entropy: Uint8Array): Promise<string> { /* ... */ }
  private async verifyIdentityHash(hash: string): Promise<boolean> { /* ... */ }
}
```

---

## References

### Standards and Specifications

1. **WebAuthn Level 3** - PRF Extension
   - Specification: https://w3c.github.io/webauthn/#prf-extension
   - Browser Support: Chrome 108+, Safari 18+, Edge 108+

2. **FIDO2 CTAP 2.1** - HMAC Secret Extension
   - Specification: https://fidoalliance.org/specs/fido-v2.1-ps-20210615/fido-client-to-authenticator-protocol-v2.1-ps-20210615.html#sctn-hmac-secret-extension

3. **RFC 5869** - HKDF (HMAC-based Extract-and-Expand Key Derivation Function)
   - https://tools.ietf.org/html/rfc5869

4. **NIST SP 800-63B** - Digital Identity Guidelines: Authentication and Lifecycle Management
   - https://pages.nist.gov/800-63-3/sp800-63b.html
   - Account Recovery: Section 6.1.2.3

5. **NIST SP 800-57** - Recommendation for Key Management
   - Part 1: General - Key Derivation and Key Wrapping

6. **BIP-39** - Mnemonic Code for Generating Deterministic Keys
   - https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki

7. **Iroh Documentation** - Peer Discovery and Protocols
   - https://docs.iroh.computer/concepts/discovery
   - https://docs.iroh.computer/concepts/protocols

### Implementation References

1. **Yubico PRF Extension Guide**
   - https://developers.yubico.com/WebAuthn/Concepts/PRF_Extension/Developers_Guide_to_PRF.html

2. **wwWallet Keystore Implementation**
   - https://github.com/wwWallet

3. **Polkadot Passkey PRF Discussion**
   - https://forum.polkadot.network/t/webauthn-passkeys-with-prf-extension-for-stateless-private-keys/14368

4. **Cove Wallet Cloud Backup Architecture**
   - https://praveenperera.com/blog/passkey-prf-bitcoin-wallet-backup

### Platform Support Matrix (as of 2025)

| Platform | Browser | Platform Authenticator | Roaming Authenticator |
|----------|---------|----------------------|---------------------|
| iOS 18+ | Safari 18+ | ✅ PRF Supported | ❌ Not Supported |
| iOS 18+ | Chrome | ✅ PRF Supported | ❌ Not Supported |
| Android | Chrome | ✅ PRF Supported | ✅ USB: Yes, NFC: No |
| macOS 15+ | Safari 18+ | ✅ PRF Supported | ❌ Not Supported |
| macOS 15+ | Chrome | ✅ PRF Supported | ✅ Supported |
| Windows 11 | Chrome/Edge | ❌ No hmac-secret | ✅ Supported |
| Windows 11 | Firefox | ❌ Limited | ✅ Supported |

---

## Document Information

- **Version**: 1.0
- **Date**: 2025
- **Author**: Systems Engineering Team
- **Classification**: Technical Documentation
- **Review Cycle**: Quarterly

---

*This document follows NIST SP 800-63B guidelines for digital identity systems and FIDO2/WebAuthn specifications for passkey-based authentication.*
