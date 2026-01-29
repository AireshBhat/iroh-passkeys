# iroh-docs Capability Delegation Design
## Identity Key ↔ Transport Key Separation for Local-First Identity Wallet

---

## Executive Summary

This document presents a comprehensive capability delegation system for iroh-docs that enables strict separation between:
- **Identity Key** (secp256k1/Ed25519): Used ONLY for signing W3C Verifiable Credentials
- **Transport Key** (Ed25519): Used for iroh-net P2P networking and iroh-docs synchronization

The design leverages UCANs (User Controlled Authorization Networks) as the primary delegation mechanism, with iroh-docs' native Namespace/Author capability system for document-level access control.

---

## 1. Architectural Overview

### 1.1 Key Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         IDENTITY WALLET KEY HIERARCHY                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────┐         ┌──────────────────────┐                  │
│  │   IDENTITY KEY       │         │   TRANSPORT KEY      │                  │
│  │   (secp256k1/Ed25519)│         │   (Ed25519)          │                  │
│  ├──────────────────────┤         ├──────────────────────┤                  │
│  │ • W3C VC Signing     │         │ • iroh-net P2P       │                  │
│  │ • DID Authentication │         │ • iroh-docs Sync     │                  │
│  │ • UCAN Issuance      │────────▶│ • Namespace Access   │                  │
│  │ • Root Authority     │  UCAN   │ • Author Operations  │                  │
│  └──────────────────────┘ Delegation └──────────────────────┘                  │
│           │                                    │                             │
│           │         ┌──────────────────┐      │                             │
│           └────────▶│  UCAN Delegation │◀─────┘                             │
│                     │  Token Chain       │                                   │
│                     └──────────────────┘                                   │
│                              │                                               │
│                              ▼                                               │
│                     ┌──────────────────┐                                   │
│                     │  iroh-docs       │                                   │
│                     │  Namespace       │                                   │
│                     │  (Write Cap)     │                                   │
│                     └──────────────────┘                                   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Trust Model

| Component | Trust Level | Compromise Impact |
|-----------|-------------|-------------------|
| Identity Key | **Critical** | Full identity takeover, VC forgery |
| Transport Key | **High** | Document manipulation, sync disruption |
| UCAN Tokens | **Medium** | Time-bounded, attenuated access |
| iroh Namespace | **Medium** | Document-level write access |

---

## 2. Capability Delegation Design

### 2.1 UCAN-Based Delegation Flow

UCANs (User Controlled Authorization Networks) provide the ideal foundation for this delegation pattern:

- **Self-certifying**: No central authority required
- **Delegable**: Chains of authority can be constructed
- **Attenuable**: Capabilities can be restricted at each delegation
- **Time-bounded**: Expiration prevents indefinite access
- **DID-based**: Compatible with W3C DID Core standards

### 2.2 Delegation Token Structure

```json
{
  "ucv": "1.0.0-rc.1",
  "iss": "did:key:z6Mkq...IdentityKey...",
  "aud": "did:key:z6Mki...TransportKey...",
  "sub": "did:key:z6Mkq...IdentityKey...",
  "nbf": 1704137004,
  "exp": 1706745600,
  "nonce": "bGlnaHQgd29yay4=",
  "att": [
    {
      "with": "iroh:namespace:z6Mkw...NamespaceId...",
      "can": "iroh/docs/write",
      "nb": {
        "key_prefix": ["trust:", "claim:", "pointer:"],
        "max_entries": 10000,
        "allowed_ops": ["insert", "update", "delete"]
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
  "prf": [],
  "meta": {
    "delegation_purpose": "identity_transport_separation",
    "wallet_version": "1.0.0",
    "device_id": "device-uuid-123"
  }
}
```

### 2.3 Capability Attenuation Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      CAPABILITY ATTENUATION HIERARCHY                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Level 0: ROOT (Identity Key)                                               │
│  ├── can: "iroh/*"                                                          │
│  ├── can: "ucan/*"                                                          │
│  └── can: "vc/sign"                                                         │
│       │                                                                      │
│       ▼                                                                      │
│  Level 1: DELEGATED (Transport Key via UCAN)                                │
│  ├── can: "iroh/docs/write"                                                 │
│  │   └── nb: {key_prefix: ["trust:", "claim:", "pointer:"]}                 │
│  ├── can: "iroh/docs/read"                                                  │
│  │   └── nb: {key_prefix: ["trust:", "claim:", "pointer:"]}                 │
│  └── can: "iroh/docs/sync"                                                  │
│       └── nb: {peers: ["*"], relay: true}                                   │
│                                                                              │
│  ATTENUATION RULES:                                                          │
│  • Each delegation MUST be equal or narrower than parent                    │
│  • Time bounds MUST be within parent's validity window                       │
│  • Resource paths MUST be sub-paths of parent's resources                    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Protocol Flow Documentation

### 3.1 Delegation Sequence

```
┌────────────────────────────────────────────────────────────────────────────────┐
│                         DELEGATION SEQUENCE DIAGRAM                             │
├────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌──────────────┐         ┌──────────────┐         ┌──────────────┐            │
│  │   Identity   │         │   Identity   │         │   Transport  │            │
│  │    Wallet    │         │     Key      │         │     Key      │            │
│  └──────┬───────┘         └──────┬───────┘         └──────┬───────┘            │
│         │                        │                        │                    │
│         │  1. Generate Transport Keypair                   │                    │
│         │───────────────────────▶│                        │                    │
│         │                        │                        │                    │
│         │  2. Create iroh-docs Namespace                   │                    │
│         │───────────────────────▶│                        │                    │
│         │                        │                        │                    │
│         │  3. Create UCAN Delegation Token                 │                    │
│         │───────────────────────▶│                        │                    │
│         │                        │                        │                    │
│         │  4. Sign UCAN with Identity Key                  │                    │
│         │───────────────────────▶│                        │                    │
│         │                        │                        │                    │
│         │  5. Export UCAN + Namespace Secret               │                    │
│         │─────────────────────────────────────────────────▶│                    │
│         │                        │                        │                    │
│         │                        │  6. Import to Transport│                    │
│         │                        │     Key Secure Storage │                    │
│         │                        │◀───────────────────────│                    │
│         │                        │                        │                    │
│         │  7. Verify UCAN Chain  │                        │                    │
│         │◀───────────────────────│                        │                    │
│         │                        │                        │                    │
│         │  8. Create iroh-docs Author                      │                    │
│         │─────────────────────────────────────────────────▶│                    │
│         │                        │                        │                    │
│         │  9. Begin Sync with Peers                        │                    │
│         │─────────────────────────────────────────────────▶│                    │
│         │                        │                        │                    │
└─────────┴────────────────────────┴────────────────────────┴────────────────────┘
```

### 3.2 Transport Key Proof of Authority

When the Transport Key needs to prove authority during iroh-docs sync:

```rust
/// Proof bundle sent during sync handshake
struct SyncAuthorityProof {
    /// The UCAN delegation token from Identity Key
    ucan_token: Ucan,
    
    /// The iroh-docs NamespaceSecret (write capability)
    namespace_secret: NamespaceSecret,
    
    /// The Author key for signing entries
    author: Author,
    
    /// Proof of possession: signature of current timestamp
    /// using Transport Key to prevent replay attacks
    proof_of_possession: Signature,
    
    /// Current timestamp for freshness
    timestamp: u64,
}

impl SyncAuthorityProof {
    /// Verify the complete authority chain
    fn verify(&self, identity_did: &Did) -> Result<(), VerificationError> {
        // 1. Verify UCAN signature and chain
        self.ucan_token.verify()?;
        
        // 2. Verify UCAN issuer matches expected Identity
        if self.ucan_token.issuer() != identity_did {
            return Err(VerificationError::WrongIssuer);
        }
        
        // 3. Verify UCAN audience matches Transport Key
        let transport_did = did_from_keypair(&self.author);
        if self.ucan_token.audience() != transport_did {
            return Err(VerificationError::WrongAudience);
        }
        
        // 4. Verify UCAN capabilities include required operations
        self.verify_capabilities()?;
        
        // 5. Verify proof of possession signature
        let message = format!("iroh-docs-auth:{}", self.timestamp);
        self.author.verify(&message, &self.proof_of_possession)?;
        
        // 6. Verify timestamp is within acceptable window (±60s)
        self.verify_timestamp_freshness()?;
        
        Ok(())
    }
}
```

### 3.3 Capability Revocation

```rust
/// Revocation mechanisms for delegated capabilities
enum RevocationStrategy {
    /// Short-lived UCANs (recommended: 24-48 hours)
    /// No explicit revocation needed - just wait for expiry
    TimeBounded {
        expiry: u64,
    },
    
    /// Explicit revocation via revocation list
    /// Identity Key signs revocation statement
    Explicit {
        revocation_ucan: Ucan,
        revocation_list_cid: Cid,
    },
    
    /// Namespace rotation - create new namespace, migrate data
    /// Old namespace becomes read-only
    NamespaceRotation {
        old_namespace: NamespaceId,
        new_namespace: NamespaceId,
        rotation_proof: Ucan,
    },
}

/// Revocation UCAN structure
struct RevocationUcan {
    iss: Did,           // Identity Key DID
    aud: Did,           // Transport Key DID (target of revocation)
    sub: Did,           // Identity Key DID
    nbf: u64,
    exp: Option<u64>,   // None = permanent revocation
    att: [{
        with: "ucan:revoke",
        can: "ucan/revoke",
        nb: {
            target_ucan_cid: Cid,  // CID of UCAN being revoked
            reason: String,
        }
    }],
    prf: [/* Original delegation UCAN */],
}
```

---

## 4. CRDT Data Model Mapping

### 4.1 Key Prefix Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    ISD (Identity as Social Intersection)                     │
│                         CRDT Key Prefix Design                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ trust: - Trust Score Graph                                             │ │
│  ├────────────────────────────────────────────────────────────────────────┤ │
│  │ Format: trust:{target_did}:{context}:{metric}                          │ │
│  │                                                                        │ │
│  │ Examples:                                                              │ │
│  │ • trust:did:key:z6Mk...:professional:reliability = 0.95               │ │
│  │ • trust:did:key:z6Mk...:social:authenticity = 0.87                    │ │
│  │ • trust:did:key:z6Mk...:financial:solvency = 0.92                     │ │
│  │                                                                        │ │
│  │ Value Structure (LWW-Register):                                        │ │
│  │ {                                                                      │ │
│  │   score: f64,           // 0.0 - 1.0                                   │ │
│  │   confidence: f64,      // 0.0 - 1.0                                   │ │
│  │   timestamp: u64,       // Unix millis                                 │ │
│  │   source: Did,          // Who issued this score                       │ │
│  │   proof_cid: Cid,       // Link to supporting evidence                 │ │
│  │ }                                                                      │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ claim: - W3C Verifiable Credentials                                    │ │
│  ├────────────────────────────────────────────────────────────────────────┤ │
│  │ Format: claim:{credential_type}:{credential_id}:{property}             │ │
│  │                                                                        │ │
│  │ Examples:                                                              │ │
│  │ • claim:UniversityDegree:3732:degree.name = "Bachelor of Science"     │ │
│  │ • claim:AgeVerification:age21:overAge = true                          │ │
│  │ • claim:ProfessionalLicense:med1234:status = "active"                 │ │
│  │                                                                        │ │
│  │ Value Structure (OR-Set for multi-issuer):                             │ │
│  │ {                                                                      │ │
│  │   credential: VerifiableCredential,  // Full VC object                 │ │
│  │   issuer: Did,                       // VC issuer                      │ │
│  │   issued_at: u64,                    // VC issuance timestamp          │ │
│  │   expires_at: Option<u64>,           // VC expiry                      │ │
│  │   proof_type: String,                // "DataIntegrity" | "JWT"        │ │
│  │ }                                                                      │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ pointer: - External References                                         │ │
│  ├────────────────────────────────────────────────────────────────────────┤ │
│  │ Format: pointer:{reference_type}:{entity_id}:{property}                │ │
│  │                                                                        │ │
│  │ Examples:                                                              │ │
│  │ • pointer:service:github:username = "alice_dev"                       │ │
│  │ • pointer:content:blog:latest_post = "ipfs://QmXyz..."                │ │
│  │ • pointer:identity:ens:name = "alice.eth"                             │ │
│  │                                                                        │ │
│  │ Value Structure (MV-Register for multi-source):                        │ │
│  │ {                                                                      │ │
│  │   uri: String,          // External reference URI                      │ │
│  │   source_type: String,  // "service", "content", "identity"            │ │
│  │   verified: bool,       // Has this been verified?                     │ │
│  │   verification_proof: Option<Cid>,                                     │ │
│  │   timestamp: u64,       // When pointer was added                      │ │
│  │ }                                                                      │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 CRDT Type Selection

| Prefix | CRDT Type | Rationale |
|--------|-----------|-----------|
| `trust:` | LWW-Register | Single value per (target, context, metric) |
| `claim:` | OR-Set | Multiple VCs for same subject from different issuers |
| `pointer:` | MV-Register | Multiple sources may reference same entity |

### 4.3 Entry Structure in iroh-docs

```rust
/// iroh-docs Entry for ISD data
struct IdentityEntry {
    /// Key with prefix (trust:, claim:, pointer:)
    key: String,
    
    /// Author = Transport Key's AuthorId
    author: AuthorId,
    
    /// Namespace = Identity-controlled namespace
    namespace: NamespaceId,
    
    /// Content hash (BLAKE3)
    content_hash: Hash,
    
    /// Content size in bytes
    content_size: u64,
    
    /// Timestamp (millis since epoch)
    timestamp: u64,
    
    /// Signature by Author key
    signature: Signature,
}

/// Content structure (serialized, then hashed)
struct IdentityContent {
    /// The actual value (CRDT operation or state)
    value: Json,
    
    /// CRDT-specific metadata
    crdt_meta: CrdtMetadata,
    
    /// Reference to UCAN proving authority
    authority_cid: Cid,
    
    /// Proof of possession signature
    proof_signature: Signature,
}

enum CrdtMetadata {
    LwwRegister { version: u64 },
    OrSet { add_wins: bool, dot: Dot },
    MvRegister { timestamps: Vec<u64> },
}
```

---

## 5. Security Analysis

### 5.1 Threat Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              THREAT MODEL                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  STRIDE Analysis:                                                            │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Spoofing                                                                ││
│  │ • T1: Attacker spoofs Identity Key DID                                  ││
│  │ • T2: Attacker spoofs Transport Key DID                                 ││
│  │ • T3: Attacker injects fake UCAN in delegation chain                    ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Tampering                                                               ││
│  │ • T4: Attacker modifies UCAN capabilities                               ││
│  │ • T5: Attacker modifies CRDT entries                                    ││
│  │ • T6: Attacker modifies sync messages                                   ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Repudiation                                                             ││
│  │ • T7: Identity Key owner denies issuing UCAN                            ││
│  │ • T8: Transport Key owner denies writing entry                          ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Information Disclosure                                                  ││
│  │ • T9: UCAN token intercepted revealing capabilities                     ││
│  │ • T10: Sync traffic analyzed for metadata                               ││
│  │ • T11: NamespaceId correlated across contexts                           ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Denial of Service                                                       ││
│  │ • T12: Flooding sync protocol with invalid entries                      ││
│  │ • T13: Revocation list poisoning                                        ││
│  │ • T14: Namespace spam (create many namespaces)                          ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │ Elevation of Privilege                                                  ││
│  │ • T15: Transport Key escalates to Identity Key capabilities             ││
│  │ • T16: Sub-delegation beyond authorized scope                           ││
│  │ • T17: Expired UCAN reused                                              ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Mitigations

| Threat | Severity | Mitigation |
|--------|----------|------------|
| T1 (Spoof Identity) | Critical | Cryptographic signature verification on all VCs and UCANs |
| T2 (Spoof Transport) | High | DID document resolution with key verification |
| T3 (Fake UCAN) | Critical | CID-based UCAN storage, signature chain validation |
| T4 (Tamper UCAN) | Critical | UCAN signature covers entire payload |
| T5 (Tamper CRDT) | High | Entry signatures by Author key, content addressing |
| T6 (Tamper Sync) | Medium | QUIC encryption, certificate pinning |
| T7 (Repudiation) | Low | Immutable UCAN chain, audit logging |
| T8 (Repudiation) | Low | Entry signatures non-repudiable |
| T9 (UCAN Leak) | Medium | Short expiry, minimal capabilities, encrypted storage |
| T10 (Traffic Analysis) | Low | Constant-size padding, noise entries |
| T11 (Correlation) | Medium | Per-context namespace rotation |
| T12 (Sync Flood) | Medium | Rate limiting, proof-of-work for entry creation |
| T13 (Revocation Poison) | High | Signed revocation statements, gossip propagation |
| T14 (Namespace Spam) | Low | Namespace creation costs, rate limiting |
| T15 (Privilege Escalation) | Critical | Capability attenuation enforcement |
| T16 (Sub-delegation) | High | UCAN `prf` chain validation, no wildcard sub-delegation |
| T17 (Expired UCAN) | Medium | Timestamp validation with clock skew tolerance |

### 5.3 Key Compromise Scenarios

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        KEY COMPROMISE SCENARIOS                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Scenario 1: Transport Key Compromised                                      │
│  ═══════════════════════════════════════                                    │
│  Impact: Attacker can write to namespace, sync with peers                   │
│  Detection: Unexpected entries, sync anomalies                              │
│  Response:                                                                  │
│    1. Revoke UCAN delegation immediately                                    │
│    2. Rotate to new Transport Key                                           │
│    3. Optionally rotate Namespace (create new, migrate)                     │
│    4. Audit entries since compromise timestamp                              │
│  Recovery: Data can be recovered from honest replicas                       │
│                                                                              │
│  Scenario 2: Identity Key Compromised                                       │
│  ════════════════════════════════════                                       │
│  Impact: FULL IDENTITY TAKEOVER - Attacker can issue any VC, any UCAN       │
│  Detection: Invalid VCs appearing, unexpected delegations                   │
│  Response:                                                                  │
│    1. IMMEDIATE: Revoke all UCANs from compromised key                      │
│    2. Publish revocation to well-known revocation lists                     │
│    3. Create NEW Identity (new DID) - cannot recover old identity           │
│    4. Re-issue all VCs under new Identity                                   │
│    5. Notify all relying parties of key rotation                            │
│  Recovery: Identity cannot be recovered - must create new                   │
│                                                                              │
│  Scenario 3: Both Keys Compromised                                          │
│  ═══════════════════════════════                                            │
│  Impact: Complete identity and data takeover                                │
│  Response: Same as Scenario 2 - create entirely new identity                │
│  Recovery: No recovery possible - start fresh                               │
│                                                                              │
│  Scenario 4: UCAN Token Leaked (Keys Safe)                                  │
│  ═════════════════════════════════════════                                  │
│  Impact: Attacker can use Transport Key capabilities until expiry           │
│  Detection: Unexpected sync activity, entry patterns                        │
│  Response:                                                                  │
│    1. Revoke specific UCAN CID                                              │
│    2. Issue new UCAN to legitimate Transport Key                            │
│    3. Audit entries created with leaked UCAN                                │
│  Recovery: Minimal - keys remain secure                                     │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Rust Code Examples

### 6.1 Capability Token Creation

```rust
use ucan::{Ucan, UcanBuilder, Capability, CapabilitySemantics};
use iroh_docs::{Author, NamespaceSecret, AuthorId, NamespaceId};
use ed25519_dalek::{SigningKey, Signature};
use serde::{Serialize, Deserialize};

/// Custom capability semantics for iroh-docs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohDocsCapability {
    /// The namespace this capability applies to
    pub namespace: NamespaceId,
    /// The operation allowed
    pub operation: IrohDocsOperation,
    /// Key prefix restrictions
    pub key_prefix: Vec<String>,
    /// Additional constraints
    pub constraints: CapabilityConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrohDocsOperation {
    Read,
    Write,
    Sync,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityConstraints {
    pub max_entries: Option<u64>,
    pub expires_at: Option<u64>,
    pub allowed_peers: Option<Vec<String>>,
}

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
    
    // Build capabilities
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
        Capability {
            with: format!("iroh:namespace:{}", namespace_id),
            can: "iroh/docs/sync".to_string(),
            nb: serde_json::json!({
                "peers": ["*"],
                "relay": true,
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
        .with_nonce(generate_nonce())
        .with_meta("delegation_purpose", "identity_transport_separation")
        .with_meta("wallet_version", "1.0.0")
        .sign(identity_key)?;
    
    Ok(ucan)
}

/// Helper: Convert signing key to DID
fn did_from_signing_key(key: &SigningKey) -> String {
    let public_key = key.verifying_key();
    // did:key format for Ed25519
    format!("did:key:z6Mk{}", base58_encode(public_key.as_bytes()))
}

/// Helper: Generate cryptographically secure nonce
fn generate_nonce() -> Vec<u8> {
    let mut nonce = [0u8; 12];
    rand::fill(&mut nonce);
    nonce.to_vec()
}
```

### 6.2 iroh-docs Author Token Delegation

```rust
use iroh_docs::{
    Author, NamespaceSecret, NamespaceId, AuthorId,
    store::{Store, Query},
};
use ed25519_dalek::{SigningKey, Signature, Signer};

/// Bundle containing all delegation artifacts
pub struct DelegationBundle {
    /// The UCAN proving authority
    pub ucan: Ucan,
    /// The namespace secret (write capability)
    pub namespace_secret: NamespaceSecret,
    /// The author for signing entries
    pub author: Author,
    /// Identity DID for verification
    pub identity_did: String,
}

/// Import delegation bundle into Transport Key's secure storage
pub fn import_delegation(
    transport_signing_key: &SigningKey,
    bundle: DelegationBundle,
) -> Result<ImportedDelegation, ImportError> {
    // 1. Verify UCAN
    bundle.ucan.verify()?;
    
    // 2. Verify UCAN audience matches our key
    let our_did = did_from_signing_key(transport_signing_key);
    if bundle.ucan.audience() != our_did {
        return Err(ImportError::WrongAudience);
    }
    
    // 3. Verify UCAN capabilities
    verify_ucan_capabilities(&bundle.ucan, &bundle.namespace_secret.id())?;
    
    // 4. Create Author from transport key
    let author = Author::from(transport_signing_key.clone());
    
    // 5. Verify author matches expected
    if author.id() != bundle.author.id() {
        return Err(ImportError::AuthorMismatch);
    }
    
    Ok(ImportedDelegation {
        ucan: bundle.ucan,
        namespace: bundle.namespace_secret,
        author,
        identity_did: bundle.identity_did,
    })
}

/// Create a signed entry with proper authority proof
pub fn create_signed_entry(
    store: &mut impl Store,
    delegation: &ImportedDelegation,
    key: &str,
    value: &[u8],
) -> Result<Entry, EntryError> {
    // 1. Verify key prefix is allowed
    verify_key_prefix(key, &delegation.ucan)?;
    
    // 2. Create content structure
    let content = IdentityContent {
        value: serde_json::from_slice(value)?,
        crdt_meta: CrdtMetadata::LwwRegister { version: 1 },
        authority_cid: delegation.ucan.to_cid(),
        proof_signature: create_proof_of_possession(&delegation.author)?,
    };
    
    // 3. Serialize and hash content
    let content_bytes = serde_json::to_vec(&content)?;
    let content_hash = blake3::hash(&content_bytes);
    
    // 4. Insert into store (this signs with Author key)
    let entry = store.insert(
        key,
        delegation.author.id(),
        delegation.namespace.id(),
        content_hash,
        content_bytes.len() as u64,
        current_timestamp(),
    )?;
    
    // 5. Store content separately (via iroh-blobs)
    store_content(content_hash, &content_bytes)?;
    
    Ok(entry)
}

/// Create proof of possession signature
fn create_proof_of_possession(author: &Author) -> Result<Signature, SignatureError> {
    let timestamp = current_timestamp();
    let message = format!("iroh-docs-auth:{}", timestamp);
    Ok(author.sign(message.as_bytes()))
}

/// Verify key prefix against UCAN capabilities
fn verify_key_prefix(key: &str, ucan: &Ucan) -> Result<(), CapabilityError> {
    let allowed_prefixes = extract_key_prefixes(ucan)?;
    
    for prefix in allowed_prefixes {
        if key.starts_with(&prefix) {
            return Ok(());
        }
    }
    
    Err(CapabilityError::KeyPrefixNotAllowed(key.to_string()))
}
```

### 6.3 UCAN Validation for Sync

```rust
use iroh_docs::net::{SyncEvent, SyncValidator};

/// Validator for incoming sync operations
pub struct DelegationSyncValidator {
    /// Expected Identity DID
    identity_did: String,
    /// Revocation list CIDs
    revocation_list: HashSet<Cid>,
    /// Clock skew tolerance (seconds)
    clock_skew_tolerance: u64,
}

impl SyncValidator for DelegationSyncValidator {
    fn validate_incoming_entry(
        &self,
        entry: &Entry,
        proof: &SyncAuthorityProof,
    ) -> Result<ValidationResult, ValidationError> {
        // 1. Verify UCAN not revoked
        let ucan_cid = proof.ucan_token.to_cid();
        if self.revocation_list.contains(&ucan_cid) {
            return Err(ValidationError::UcanRevoked);
        }
        
        // 2. Verify UCAN chain
        proof.verify(&self.identity_did)?;
        
        // 3. Verify author matches UCAN audience
        if entry.author() != proof.author.id() {
            return Err(ValidationError::AuthorMismatch);
        }
        
        // 4. Verify namespace matches UCAN
        if entry.namespace() != proof.namespace_secret.id() {
            return Err(ValidationError::NamespaceMismatch);
        }
        
        // 5. Verify entry signature
        entry.verify_signature(&proof.author.public_key())?;
        
        // 6. Verify content hash matches
        let content = fetch_content(entry.content_hash())?;
        let computed_hash = blake3::hash(&content);
        if computed_hash != entry.content_hash() {
            return Err(ValidationError::ContentHashMismatch);
        }
        
        // 7. Verify content structure and authority proof
        let identity_content: IdentityContent = serde_json::from_slice(&content)?;
        self.verify_content_proof(&identity_content, proof)?;
        
        Ok(ValidationResult::Valid)
    }
}

impl DelegationSyncValidator {
    fn verify_content_proof(
        &self,
        content: &IdentityContent,
        proof: &SyncAuthorityProof,
    ) -> Result<(), ValidationError> {
        // Verify the content's authority CID matches the UCAN
        if content.authority_cid != proof.ucan_token.to_cid() {
            return Err(ValidationError::AuthorityMismatch);
        }
        
        // Verify proof of possession signature
        let timestamp = extract_timestamp(&content.proof_signature)?;
        let message = format!("iroh-docs-auth:{}", timestamp);
        
        proof.author.verify(message.as_bytes(), &content.proof_signature)
            .map_err(|_| ValidationError::InvalidProofOfPossession)?;
        
        // Verify timestamp freshness
        let now = current_timestamp();
        if timestamp.abs_diff(now) > self.clock_skew_tolerance {
            return Err(ValidationError::StaleProof);
        }
        
        Ok(())
    }
}
```

### 6.4 Complete Integration Example

```rust
use iroh::{Endpoint, protocol::Router};
use iroh_blobs::{BlobsProtocol, store::mem::MemStore};
use iroh_docs::{protocol::Docs, ALPN as DOCS_ALPN};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN};

/// Complete setup for Identity Wallet with capability delegation
pub async fn setup_identity_wallet(
    identity_key: SigningKey,
) -> anyhow::Result<IdentityWallet> {
    // 1. Generate Transport Key (ephemeral, per-device)
    let transport_key = generate_transport_key();
    
    // 2. Create iroh-docs namespace
    let namespace = NamespaceSecret::new(&mut rand::thread_rng());
    
    // 3. Create UCAN delegation
    let ucan = create_delegation_ucan(
        &identity_key,
        &transport_key,
        &namespace,
        48, // 48-hour validity
    )?;
    
    // 4. Create Author from transport key
    let author = Author::from(transport_key.clone());
    
    // 5. Setup iroh networking
    let endpoint = Endpoint::builder()
        .bind()
        .await?;
    
    let gossip = Gossip::builder()
        .spawn(endpoint.clone());
    
    let blobs = MemStore::default();
    
    let docs = Docs::memory()
        .spawn(endpoint.clone(), blobs.clone(), gossip.clone())
        .await?;
    
    // 6. Create router
    let router = Router::builder(endpoint.clone())
        .accept(GOSSIP_ALPN, gossip)
        .accept(DOCS_ALPN, docs.clone())
        .spawn();
    
    // 7. Import namespace and author into docs
    let replica = docs.create_replica(namespace.id()).await?;
    replica.set_author(author).await?;
    
    // 8. Store UCAN securely
    let secure_storage = SecureStorage::new(&transport_key);
    secure_storage.store_ucan(&ucan).await?;
    
    Ok(IdentityWallet {
        identity_key,
        transport_key,
        namespace,
        author,
        ucan,
        docs,
        router,
        secure_storage,
    })
}

/// Write identity data with proper capability proof
pub async fn write_identity_data(
    wallet: &IdentityWallet,
    key: &str,
    value: serde_json::Value,
) -> anyhow::Result<()> {
    // Verify key prefix
    if !key.starts_with("trust:") && !key.starts_with("claim:") && !key.starts_with("pointer:") {
        return Err(anyhow!("Invalid key prefix"));
    }
    
    // Create content with UCAN proof
    let content = IdentityContent {
        value,
        crdt_meta: CrdtMetadata::LwwRegister { version: 1 },
        authority_cid: wallet.ucan.to_cid(),
        proof_signature: create_proof_of_possession(&wallet.author)?,
    };
    
    // Serialize and store
    let content_bytes = serde_json::to_vec(&content)?;
    let content_hash = blake3::hash(&content_bytes);
    
    // Insert into replica
    wallet.docs.insert(
        key,
        wallet.author.id(),
        wallet.namespace.id(),
        content_hash,
        content_bytes.len() as u64,
        current_timestamp(),
    ).await?;
    
    Ok(())
}
```

---

## 7. References

### 7.1 Specifications

- [UCAN Specification](https://github.com/ucan-wg/spec) - User Controlled Authorization Networks
- [W3C DID Core v1.0](https://www.w3.org/TR/did-core/) - Decentralized Identifiers
- [W3C Verifiable Credentials Data Model v2.0](https://www.w3.org/TR/vc-data-model-2.0/) - VC Data Model
- [Macaroons Paper](https://research.google/pubs/macaroons-cookies-with-contextual-caveats-for-decentralized-authorization-in-the-cloud/) - Google Research
- [iroh-docs Documentation](https://docs.rs/iroh-docs/) - Rust API Documentation

### 7.2 Related Work

- **ZCAP-LD**: Linked Data capabilities for decentralized authorization
- **SPKI/SDSI**: Simple Public Key Infrastructure (predecessor to UCAN)
- **CACAO**: Chain-Agnostic CApability Objects (blockchain-based)
- **Biscuit**: Datalog-based authorization tokens
- **Local-First Auth**: CRDT-based group membership

### 7.3 Cryptographic Libraries

- `ed25519-dalek` - Ed25519 signatures in Rust
- `ucan-rs` - UCAN implementation in Rust
- `iroh-docs` - CRDT document synchronization
- `blake3` - Content addressing hash function

---

## 8. Appendix

### 8.1 UCAN Verification Pseudocode

```
function verifyUCAN(ucan, expectedIssuer, expectedAudience):
    // 1. Verify signature
    if !verifySignature(ucan, ucan.iss):
        return false
    
    // 2. Verify time bounds
    now = currentTime()
    if ucan.nbf && now < ucan.nbf:
        return false
    if ucan.exp && now > ucan.exp:
        return false
    
    // 3. Verify issuer matches expected
    if ucan.iss != expectedIssuer:
        return false
    
    // 4. Verify audience matches expected
    if ucan.aud != expectedAudience:
        return false
    
    // 5. Verify proof chain (if any)
    for proofCID in ucan.prf:
        proofUCAN = resolveCID(proofCID)
        if !verifyUCAN(proofUCAN, proofUCAN.iss, ucan.iss):
            return false
        
        // Verify capabilities attenuate properly
        if !verifyAttenuation(ucan.att, proofUCAN.att):
            return false
    
    return true
```

### 8.2 Glossary

| Term | Definition |
|------|------------|
| **UCAN** | User Controlled Authorization Network - decentralized capability tokens |
| **DID** | Decentralized Identifier - self-sovereign identity identifier |
| **VC** | Verifiable Credential - W3C standard for signed claims |
| **CRDT** | Conflict-free Replicated Data Type - eventually consistent data structure |
| **Namespace** | iroh-docs write capability (identified by NamespaceId) |
| **Author** | iroh-docs signing key for entries (identified by AuthorId) |
| **Attenuation** | Restricting capabilities when delegating |
| **CID** | Content Identifier - hash-based content addressing |

---

*Document Version: 1.0*
*Last Updated: 2025*
*Status: Design Specification*
