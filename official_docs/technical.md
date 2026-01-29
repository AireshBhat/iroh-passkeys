# Technical Architecture: Iroh Personal Node (Mobile Browser MVP)

## Document Information
- **Version**: 1.0.0
- **Status**: Draft
- **Last Updated**: 2026-01-29
- **Scope**: Minimum Viable Product for browser-based Iroh node

---

## 1. Executive Summary

This document describes the technical architecture for running a personal Iroh node within a mobile web browser. The system enables users to participate in the Iroh peer-to-peer network directly from their phone's browser using WebAssembly, with keys derived from WebAuthn PRF (Passkeys).

### Key Constraints
- Browser sandbox prevents direct UDP/TCP socket access
- All connections MUST flow through Iroh relay servers
- Tab must remain active for node operation
- WebAuthn PRF availability varies by platform

---

## 2. System Architecture

### 2.1 High-Level Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          USER INTERFACE LAYER                                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  Presentation Components                                             │   │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌─────────────┐ │   │
│  │  │  Dashboard   │ │   Document   │ │    Peer      │ │   Settings  │ │   │
│  │  │   Status     │ │    Manager   │ │  Connection  │ │   & Config  │ │   │
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └─────────────┘ │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────────────┤
│                      JAVASCRIPT/WASM BRIDGE LAYER                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  wasm-bindgen Interface                                              │   │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌─────────────┐ │   │
│  │  │   Key Der.   │ │  Node Mgmt   │ │  Document    │ │   Network   │ │   │
│  │  │    (HKDF)    │ │   (Init/Stop)│ │    Ops       │ │   Events    │ │   │
│  │  └──────────────┘ └──────────────┘ └──────────────┘ └─────────────┘ │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────────────┤
│                         RUST/WASM CORE LAYER                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │                    Key Management Module                      │  │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │  │   │
│  │  │  │ WebAuthn    │  │    HKDF     │  │   Transport Key     │   │  │   │
│  │  │  │ PRF Input   │──│   SHA-256   │──│  (Ed25519 Seed)     │   │  │   │
│  │  │  │ (32 bytes)  │  │  Extract/   │  │                     │   │  │   │
│  │  │  │             │  │   Expand    │  │                     │   │  │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │                                                                    │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │                    Iroh Node Module                           │  │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │  │   │
│  │  │  │  Endpoint   │  │   Secret    │  │   Relay Client      │   │  │   │
│  │  │  │   Builder   │──│    Key      │──│  (WebTransport/     │   │  │   │
│  │  │  │             │  │  (Ed25519)  │  │   WebSocket)        │   │  │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │  │   │
│  │  │                              │                               │  │   │
│  │  │                              ▼                               │  │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │  │   │
│  │  │  │ NodeAddr    │  │  Discovery  │  │   Connection Mgmt   │   │  │   │
│  │  │  │  (Relay)    │  │   (N0)      │  │   (QUIC over Web)   │   │  │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  │                                                                    │   │
│  │  ┌──────────────────────────────────────────────────────────────┐  │   │
│  │  │                 iroh-docs Module (Future)                     │  │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │  │   │
│  │  │  │ Namespace   │  │   Author    │  │   CRDT Operations   │   │  │   │
│  │  │  │  (Write     │  │  (Entry      │  │  (trust:/claim:/    │   │  │   │
│  │  │  │   Cap)      │  │   Signer)    │  │   pointer: keys)    │   │  │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │  │   │
│  │  └──────────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────────────┤
│                       BROWSER PLATFORM LAYER                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ WebAuthn API │  │   WebAssembly │  │  IndexedDB   │  │   WebTransport  │  │
│  │ (PRF Ext.)   │  │    Runtime    │  │  (Storage)   │  │   / WebSocket   │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         NETWORK LAYER                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Iroh Relay Server                                │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐  │   │
│  │  │  DERP       │  │  Address    │  │    Encrypted Forwarding     │  │   │
│  │  │  Protocol   │  │  Mapping    │  │    (E2E Encrypted)          │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                       │                                      │
│                                       ▼                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Iroh P2P Network                             │   │
│  │                    (Other Nodes: Native/WASM)                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Component Breakdown

#### 2.2.1 Key Management Module

**Purpose**: Derive cryptographic keys from WebAuthn PRF output using HKDF-SHA256.

| Component | Responsibility | Input | Output |
|-----------|---------------|-------|--------|
| PRF Receiver | Receive 32-byte PRF result from WebAuthn | Credential assertion | Master seed (32 bytes) |
| HKDF Extract | Extract pseudorandom key from IKM | Master seed + Salt | PRK (32 bytes) |
| HKDF Expand | Derive specific keys with info strings | PRK + Info string | Key material (32 bytes) |
| Key Container | Securely hold derived keys | Key material | Typed key structs |

**Key Derivation Flow**:
```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  WebAuthn PRF   │────►│   HKDF-SHA256   │────►│  Transport Key  │
│  (32 bytes)     │     │                 │     │  (Ed25519 seed) │
└─────────────────┘     │  Salt: "Local   │     └─────────────────┘
                        │  FirstIdentity  │              │
                        │  Wallet_v1_2024"│              ▼
                        │                 │     ┌─────────────────┐
                        │  Info: "identity│────►│  Future: Id Key │
                        │  -wallet/v1/    │     │  (secp256k1)    │
                        │  transport-key" │     └─────────────────┘
                        └─────────────────┘
```

#### 2.2.2 Iroh Node Module

**Purpose**: Initialize and manage the Iroh endpoint for P2P networking.

| Component | Responsibility |
|-----------|---------------|
| Endpoint Builder | Configure relay mode, discovery, ALPN |
| Secret Key Manager | Hold Ed25519 private key for node identity |
| Relay Client | Maintain connection to relay server (WebTransport fallback) |
| Discovery Client | Register/query node addresses via N0 discovery |
| Connection Pool | Manage active QUIC connections |

**Node Initialization Flow**:
```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Transport Key  │────►│  Endpoint       │────►│  Relay          │
│  (32 bytes)     │     │  Builder        │     │  Connection     │
└─────────────────┘     │                 │     └─────────────────┘
                        │  - relay_mode:  │              │
                        │    Enabled      │              ▼
                        │  - discovery:   │     ┌─────────────────┐
                        │    N0           │────►│  Node ID        │
                        │  - bind()       │     │  (Public Key)   │
                        └─────────────────┘     └─────────────────┘
```

#### 2.2.3 Document Synchronization Module (Future Phase)

**Purpose**: Enable CRDT-based document synchronization via iroh-docs.

| Component | Responsibility |
|-----------|---------------|
| Namespace Manager | Create/open document namespaces |
| Author Identity | Sign document entries |
| Sync Engine | Handle inbound/outbound sync protocols |
| Key-Value Store | Store document entries with prefixes |

---

## 3. Data Flow

### 3.1 Authentication & Key Derivation Flow

```
┌─────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   User  │────►│   Browser   │────►│  WebAuthn   │────►│  Platform   │
│ (Touch) │     │   (JS)      │     │   API       │     │  Auth (SE)  │
└─────────┘     └─────────────┘     └─────────────┘     └──────┬──────┘
     │                                                          │
     │                    PRF Output (32 bytes)                  │
     │◄─────────────────────────────────────────────────────────┘
     │
     ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  HKDF-SHA256 │────►│  Transport  │────►│  Store in   │
│  Derivation │     │  Key (32B)  │     │  WASM Mem   │
└─────────────┘     └─────────────┘     └─────────────┘
```

### 3.2 Node Startup Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  User Clicks │────►│  WASM init  │────►│  SecretKey  │────►│  Endpoint   │
│  "Start Node"│     │  _node()    │     │  from_bytes │     │  Builder    │
└─────────────┘     └─────────────┘     └─────────────┘     └──────┬──────┘
                                                                    │
                    ┌─────────────┐     ┌─────────────┐            │
                    │  Node ID    │◄────│  Bind ()    │◄───────────┘
                    │  (Display)  │     │  (async)    │
                    └─────────────┘     └─────────────┘
                                               │
                                               ▼
                    ┌─────────────┐     ┌─────────────┐
                    │  Discovery  │◄────│  Relay      │
                    │  Register   │     │  Connect    │
                    └─────────────┘     └─────────────┘
```

### 3.3 Peer Connection Flow (Relay-Based)

```
┌─────────────────┐                    ┌─────────────────┐
│   Browser Node  │                    │   Target Node   │
│   (This Device) │                    │   (Remote)      │
└────────┬────────┘                    └────────┬────────┘
         │                                      │
         │ 1. Query relay for node address      │
         ├─────────────────────────────────────►│
         │ 2. Receive relay URL + node ID       │
         │◄─────────────────────────────────────┤
         │                                      │
         │ 3. Open WebTransport/WebSocket       │
         │    to relay                          │
         ├─────────────────────┐                │
         │                     │                │
         │                     ▼                │
         │            ┌─────────────────┐       │
         │            │   Iroh Relay    │       │
         │            │   Server        │       │
         │            └────────┬────────┘       │
         │                     │                │
         │ 4. Relay forwards   │                │
         │    encrypted packets│                │
         │◄────────────────────┴────────────────►│
         │    (E2E encrypted, relay cannot       │
         │     decrypt payload)                  │
         │                                      │
         │ 5. QUIC connection established       │
         │    over relay                        │
         ════════════════════════════════════════
```

### 3.4 Document Sync Flow (Future)

```
┌─────────────────┐                    ┌─────────────────┐
│   Browser Node  │                    │   Peer Node     │
│   (Your Device) │                    │   (Collaborator)│
└────────┬────────┘                    └────────┬────────┘
         │                                      │
         │ 1. Open document namespace           │
         │    (write capability required)       │
         ├─────────────────────────────────────►│
         │                                      │
         │ 2. Sync protocol handshake           │
         ├─────────────────────────────────────►│
         │◄─────────────────────────────────────┤
         │                                      │
         │ 3. Exchange entry vectors            │
         │    (CRDT merge)                      │
         ├─────────────────────────────────────►│
         │◄─────────────────────────────────────┤
         │                                      │
         │ 4. Request missing entries           │
         ├─────────────────────────────────────►│
         │◄─────────────────────────────────────┤
         │                                      │
         │ 5. Local document updated            │
         │    (UI reactive update)              │
         ▼                                      ▼
```

---

## 4. Interfaces

### 4.1 WASM-JavaScript Interface

```typescript
// Key Derivation
function derive_transport_key(master_seed: Uint8Array): Uint8Array;

// Node Management
class IrohNode {
  constructor(transport_key: Uint8Array): Promise<IrohNode>;
  get node_id(): string;
  is_connected(): boolean;
  shutdown(): Promise<void>;
  connect(node_addr: string): Promise<Connection>;
}

// Connection
class Connection {
  close(): Promise<void>;
  // Future: send/recv streams
}

// Events (wasm-bindgen callbacks)
interface NodeEvents {
  on_peer_connected(peer_id: string): void;
  on_peer_disconnected(peer_id: string): void;
  on_connection_error(error: string): void;
}
```

### 4.2 WebAuthn Interface

```typescript
interface PublicKeyCredentialRequestOptions {
  challenge: Uint8Array;
  rpId: string;
  allowCredentials: PublicKeyCredentialDescriptor[];
  userVerification: 'required' | 'preferred' | 'discouraged';
  extensions: {
    prf?: {
      eval: {
        first: Uint8Array;  // 32-byte salt
      }
    }
  }
}

interface AuthenticationExtensionsPRFOutputs {
  prf?: {
    results?: {
      first?: ArrayBuffer;  // 32-byte output
    }
  }
}
```

### 4.3 External Service Interfaces

| Service | Protocol | Purpose | Endpoint |
|---------|----------|---------|----------|
| Iroh Relay | WebTransport/WebSocket | Connection relay | `wss://relay.iroh.network` |
| N0 Discovery | HTTPS | Node address lookup | `https://iroh.network` |

---

## 5. Security Architecture

### 5.1 Threat Model

| Threat | Likelihood | Impact | Mitigation |
|--------|-----------|--------|------------|
| XSS stealing keys | Medium | Critical | Keys only in WASM memory, never JS |
| Relay compromise | Low | Medium | E2E encryption, relay cannot decrypt |
| PRF side-channel | Low | High | Hardware-backed PRF (Secure Enclave/TPM) |
| Man-in-the-middle | Medium | High | QUIC crypto, certificate pinning |
| Browser exploit | Low | Critical | Short-lived keys, minimal attack surface |

### 5.2 Key Security

```
┌─────────────────────────────────────────────────────────────┐
│                    KEY LIFECYCLE                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   Generation          Storage           Usage              │
│   ──────────          ───────           ─────              │
│                                                              │
│   WebAuthn PRF    →   WASM Linear    →   Iroh Endpoint     │
│   (Hardware)          Memory only        (In-memory only)    │
│                       (No persistence)                       │
│                                                              │
│   Zeroization:                                               │
│   - After key derivation                                     │
│   - On node shutdown                                         │
│   - On tab unload                                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 5.3 Transport Security

- **QUIC**: All connections use QUIC protocol with TLS 1.3
- **Relay Encryption**: End-to-end encrypted, relay forwards ciphertext only
- **Key Derivation**: HKDF-SHA256 per RFC 5869

---

## 6. State Management

### 6.1 Application States

```
                    ┌─────────────┐
         ┌─────────►│   IDLE      │◄────────┐
         │          │  (no keys)  │         │
         │          └──────┬──────┘         │
         │                 │ authenticate() │
         │                 ▼                │
         │          ┌─────────────┐         │
         │          │ AUTHENTICATED│        │
         │          │ (keys derived)│       │
         │          └──────┬──────┘         │
         │                 │ init_node()    │
         │                 ▼                │
         │          ┌─────────────┐         │
         │    ┌────►│ CONNECTING  │────┐    │
         │    │     │  (to relay) │    │    │
         │    │     └──────┬──────┘    │    │
         │    │            │ success   │    │
         │    │            ▼           │    │
         │    │     ┌─────────────┐    │    │
         │    └─────┤   ONLINE    │◄───┘    │
         │    error │  (relay)    │─────────┘
         │◄─────────┤             │ shutdown()
         │          └──────┬──────┘
         │                 │ error
         │                 ▼
         │          ┌─────────────┐
         └──────────┤    ERROR    │
                    │  (retryable)│
                    └─────────────┘
```

### 6.2 State Transitions

| From | To | Trigger | Action |
|------|----|---------|--------|
| IDLE | AUTHENTICATED | User authenticates with Passkey | Derive keys, store in WASM |
| AUTHENTICATED | CONNECTING | User clicks "Start Node" | Initialize Iroh endpoint |
| CONNECTING | ONLINE | Relay connection succeeds | Update UI, enable features |
| CONNECTING | ERROR | Connection fails | Show error, allow retry |
| ONLINE | IDLE | User clicks "Shutdown" | Close endpoint, zeroize keys |
| * | IDLE | Tab unload | Emergency cleanup |

---

## 7. Performance Considerations

### 7.1 Resource Usage

| Resource | Expected Usage | Optimization |
|----------|---------------|--------------|
| Memory | 10-50 MB | Zeroize unused buffers |
| CPU | Low (idle), Medium (sync) | Efficient CRDT algorithms |
| Battery | ~5%/hour | Batch sync operations |
| Network | ~1 KB/hour (keepalive) | Compress where possible |

### 7.2 Browser Constraints

| Constraint | Impact | Mitigation |
|------------|--------|------------|
| Tab throttling | Node pauses when hidden | Service Worker for wake |
| Memory limits | ~200 MB typical | Streaming for large docs |
| Storage quotas | ~60 MB default | Request persistent storage |
| Background sync | Limited | Push notification trigger |

---

## 8. Error Handling

### 8.1 Error Categories

| Category | Examples | Recovery Strategy |
|----------|----------|-------------------|
| Auth Errors | PRF unsupported, user cancel | Fallback to demo mode |
| Network Errors | Relay unreachable, timeout | Exponential backoff retry |
| Crypto Errors | Key derivation failure | Fatal, require re-auth |
| Protocol Errors | Invalid node address | User feedback, retry |

### 8.2 Error Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Error Detected │────►│  Classify Error │────►│  Select Strategy│
│   (WASM/JS)      │     │                 │     │                 │
└─────────────────┘     └────────┬────────┘     └────────┬────────┘
                                 │                       │
                    ┌────────────┼────────────┐          │
                    ▼            ▼            ▼          ▼
            ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
            │   Auth   │  │ Network  │  │  Crypto  │  │  Fatal   │
            │   Fail   │  │   Fail   │  │   Fail   │  │   Error  │
            └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘
                 │             │             │             │
                 ▼             ▼             ▼             ▼
            ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
            │  Offer   │  │  Retry   │  │  Clear   │  │  Show    │
            │ Fallback │  │  with    │  │  State   │  │  Error   │
            │          │  │ Backoff  │  │  & Reset │  │  & Reset │
            └──────────┘  └──────────┘  └──────────┘  └──────────┘
```

---

## 9. Future Enhancements

### 9.1 Phase 2: iroh-docs Integration
- Document CRDT synchronization
- Namespace creation/opening
- Key prefix support (trust:/claim:/pointer:)

### 9.2 Phase 3: UCAN Delegation
- Capability token generation
- Identity key separation
- Delegation chain validation

### 9.3 Phase 4: Mobile Optimizations
- Service Worker for background sync
- Push notification integration
- Native app wrapper (Capacitor/Tauri)

---

## 10. References

- [Iroh Documentation](https://iroh.computer/docs)
- [WebAuthn PRF Extension](https://w3c.github.io/webauthn/#prf-extension)
- [HKDF RFC 5869](https://tools.ietf.org/html/rfc5869)
- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- [iroh WASM Browser Support](https://docs.iroh.computer/deployment/wasm-browser-support)
