# Code Examples: Iroh Mobile Integration

## Table of Contents
1. [UniFFi UDL Interface](#1-uniffi-udl-interface)
2. [Rust Implementation](#2-rust-implementation)
3. [React Native Module](#3-react-native-module)
4. [iOS Background Execution](#4-ios-background-execution)
5. [Android Background Execution](#5-android-background-execution)
6. [Push Notification Server](#6-push-notification-server)

---

## 1. UniFFi UDL Interface

### 1.1 Complete Iroh Wallet UDL

```idl
// iroh_wallet.udl - Complete UniFFi Interface Definition

namespace iroh_wallet {
    // Core node operations
    [Async, Throws=WalletError]
    NodeHandle init_node(NodeConfig config);
    
    [Throws=WalletError]
    NodeInfo get_node_info(NodeHandle handle);
    
    [Async, Throws=WalletError]
    void shutdown_node(NodeHandle handle);
    
    // Connection management
    [Async, Throws=WalletError]
    ConnectionHandle connect(NodeHandle node, string node_addr);
    
    [Async, Throws=WalletError]
    void disconnect(ConnectionHandle conn);
    
    // Document operations
    [Async, Throws=WalletError]
    DocumentHandle open_document(NodeHandle node, string namespace_id);
    
    [Async, Throws=WalletError]
    void close_document(DocumentHandle doc);
    
    [Async, Throws=WalletError]
    void write_entry(DocumentHandle doc, string key, sequence<u8> value);
    
    [Async, Throws=WalletError]
    sequence<u8> read_entry(DocumentHandle doc, string key);
    
    [Async, Throws=WalletError]
    sequence<Entry> get_entries(DocumentHandle doc);
    
    // Sync operations
    [Async, Throws=WalletError]
    SyncResult sync_document(DocumentHandle doc, SyncOptions options);
    
    // Subscription
    [Async, Throws=WalletError]
    SubscriptionHandle subscribe(DocumentHandle doc, DocumentCallback callback);
    
    [Throws=WalletError]
    void unsubscribe(SubscriptionHandle sub);
    
    // Identity operations
    [Throws=WalletError]
    Identity create_identity(NodeHandle node, string name);
    
    [Throws=WalletError]
    Identity get_identity(NodeHandle node, string id);
    
    [Async, Throws=WalletError]
    Signature sign_data(NodeHandle node, string identity_id, sequence<u8> data);
    
    [Async, Throws=WalletError]
    boolean verify_signature(NodeHandle node, string identity_id, sequence<u8> data, Signature sig);
    
    // Relay operations
    [Throws=WalletError]
    void add_relay(NodeHandle node, string relay_url);
    
    [Throws=WalletError]
    void remove_relay(NodeHandle node, string relay_url);
    
    [Throws=WalletError]
    sequence<RelayInfo> get_relays(NodeHandle node);
};

// Configuration
dictionary NodeConfig {
    sequence<string> relay_urls;
    boolean enable_discovery;
    string? alpn_protocol;
    u32 max_connections;
    u32 idle_timeout_secs;
};

// Node information
dictionary NodeInfo {
    string node_id;
    sequence<string> relay_addresses;
    boolean is_connected;
    ConnectionStats stats;
    NodeState state;
};

// Connection statistics
dictionary ConnectionStats {
    u64 bytes_sent;
    u64 bytes_received;
    u32 active_connections;
    u32 pending_connections;
    u64 total_connections;
};

// Sync options
dictionary SyncOptions {
    boolean bidirectional;
    u32 timeout_secs;
    sequence<string>? filter_prefixes;
};

// Sync result
dictionary SyncResult {
    boolean success;
    u32 entries_synced;
    u32 entries_failed;
    string? error_message;
};

// Entry in a document
dictionary Entry {
    string key;
    sequence<u8> value;
    string author;
    u64 timestamp;
};

// Identity
dictionary Identity {
    string id;
    string name;
    string public_key;
    u64 created_at;
};

// Signature
dictionary Signature {
    sequence<u8> data;
    string algorithm;
    u64 timestamp;
};

// Relay information
dictionary RelayInfo {
    string url;
    boolean is_connected;
    u64 latency_ms;
    u64 bytes_sent;
    u64 bytes_received;
};

// Node state enum
enum NodeState {
    "Initializing",
    "Connecting",
    "Connected",
    "Syncing",
    "Disconnecting",
    "Disconnected",
    "Error"
};

// Error types
[Error]
enum WalletError {
    "NodeInitFailed",
    "ConnectionFailed",
    "DocumentNotFound",
    "SyncFailed",
    "InvalidKey",
    "NetworkError",
    "Timeout",
    "InvalidAddress",
    "IdentityNotFound",
    "SignatureFailed",
    "RelayError",
    "AlreadyInitialized",
    "NotInitialized"
};

// Opaque handle types
interface NodeHandle {};
interface ConnectionHandle {};
interface DocumentHandle {};
interface SubscriptionHandle {};

// Callback interface for document changes
callback interface DocumentCallback {
    [Async]
    void on_entry_changed(string key, sequence<u8> value, string author);
    
    [Async]
    void on_peer_joined(string peer_id);
    
    [Async]
    void on_peer_left(string peer_id);
    
    [Async]
    void on_sync_started();
    
    [Async]
    void on_sync_completed(u32 entries_synced);
    
    [Async]
    void on_error(string error);
};
```

---

## 2. Rust Implementation

### 2.1 Cargo.toml

```toml
[package]
name = "iroh-wallet-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib", "cdylib"]
name = "iroh_wallet_core"

[dependencies]
# Iroh dependencies
iroh = "0.28"
iroh-docs = "0.28"
iroh-gossip = "0.28"
iroh-blobs = "0.28"

# UniFFi
uniffi = { version = "0.28", features = ["tokio"] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "1"
anyhow = "1"

# Crypto
ed25519-dalek = "2"
rand = "0.8"

# Utilities
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"

[build-dependencies]
uniffi = { version = "0.28", features = ["build"] }
```

### 2.2 build.rs

```rust
// build.rs
fn main() {
    uniffi::generate_scaffolding("src/iroh_wallet.udl")
        .expect("Failed to generate scaffolding");
}
```

### 2.3 Core Implementation

```rust
// src/lib.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use iroh::{Endpoint, RelayMode, NodeAddr, Connection};
use iroh_docs::{Doc, Author, NamespaceId, Store};
use tokio::sync::{RwLock, mpsc};
use uniffi::deps::log::info;

uniffi::setup_scaffolding!();

// ============== Error Types ==============

#[derive(uniffi::Error, Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Node initialization failed: {0}")]
    NodeInitFailed(String),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Document not found: {0}")]
    DocumentNotFound(String),
    
    #[error("Sync failed: {0}")]
    SyncFailed(String),
    
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Operation timed out")]
    Timeout,
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Identity not found: {0}")]
    IdentityNotFound(String),
    
    #[error("Signature failed: {0}")]
    SignatureFailed(String),
    
    #[error("Relay error: {0}")]
    RelayError(String),
    
    #[error("Node already initialized")]
    AlreadyInitialized,
    
    #[error("Node not initialized")]
    NotInitialized,
}

// ============== Configuration Types ==============

#[derive(uniffi::Record, Clone, Debug)]
pub struct NodeConfig {
    pub relay_urls: Vec<String>,
    pub enable_discovery: bool,
    pub alpn_protocol: Option<String>,
    pub max_connections: u32,
    pub idle_timeout_secs: u32,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            relay_urls: vec![
                "https://relay.iroh.network".to_string(),
            ],
            enable_discovery: true,
            alpn_protocol: Some("iroh-wallet/1".to_string()),
            max_connections: 10,
            idle_timeout_secs: 300,
        }
    }
}

// ============== Information Types ==============

#[derive(uniffi::Record, Clone, Debug)]
pub struct NodeInfo {
    pub node_id: String,
    pub relay_addresses: Vec<String>,
    pub is_connected: bool,
    pub stats: ConnectionStats,
    pub state: NodeState,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct ConnectionStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active_connections: u32,
    pub pending_connections: u32,
    pub total_connections: u64,
}

#[derive(uniffi::Enum, Clone, Debug)]
pub enum NodeState {
    Initializing,
    Connecting,
    Connected,
    Syncing,
    Disconnecting,
    Disconnected,
    Error,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct SyncOptions {
    pub bidirectional: bool,
    pub timeout_secs: u32,
    pub filter_prefixes: Option<Vec<String>>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct SyncResult {
    pub success: bool,
    pub entries_synced: u32,
    pub entries_failed: u32,
    pub error_message: Option<String>,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct Entry {
    pub key: String,
    pub value: Vec<u8>,
    pub author: String,
    pub timestamp: u64,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub public_key: String,
    pub created_at: u64,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct Signature {
    pub data: Vec<u8>,
    pub algorithm: String,
    pub timestamp: u64,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct RelayInfo {
    pub url: String,
    pub is_connected: bool,
    pub latency_ms: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

// ============== Opaque Handle Types ==============

#[derive(uniffi::Object)]
pub struct NodeHandle {
    endpoint: Arc<RwLock<Endpoint>>,
    docs_store: Arc<RwLock<Store>>,
    shutdown_tx: mpsc::Sender<()>,
    state: Arc<RwLock<NodeState>>,
    connections: Arc<RwLock<HashMap<String, Connection>>>,
}

#[derive(uniffi::Object)]
pub struct ConnectionHandle {
    id: String,
    connection: Arc<RwLock<Connection>>,
}

#[derive(uniffi::Object)]
pub struct DocumentHandle {
    id: String,
    doc: Arc<RwLock<Doc>>,
}

#[derive(uniffi::Object)]
pub struct SubscriptionHandle {
    id: String,
    cancel_tx: mpsc::Sender<()>,
}

// ============== Callback Interface ==============

#[uniffi::export(callback_interface)]
pub trait DocumentCallback: Send + Sync {
    fn on_entry_changed(&self, key: String, value: Vec<u8>, author: String);
    fn on_peer_joined(&self, peer_id: String);
    fn on_peer_left(&self, peer_id: String);
    fn on_sync_started(&self);
    fn on_sync_completed(&self, entries_synced: u32);
    fn on_error(&self, error: String);
}

// ============== NodeHandle Implementation ==============

#[uniffi::export(async_runtime = "tokio")]
impl NodeHandle {
    #[uniffi::constructor]
    pub async fn new(config: NodeConfig) -> Result<Arc<Self>, WalletError> {
        let relay_mode = if config.relay_urls.is_empty() {
            RelayMode::Disabled
        } else {
            let urls: Vec<_> = config.relay_urls.iter()
                .filter_map(|url| url.parse().ok())
                .collect();
            RelayMode::Custom(urls)
        };

        let endpoint = Endpoint::builder()
            .relay_mode(relay_mode)
            .discovery(config.enable_discovery)
            .bind()
            .await
            .map_err(|e| WalletError::NodeInitFailed(e.to_string()))?;

        // Initialize document store
        let docs_store = Store::memory()
            .map_err(|e| WalletError::NodeInitFailed(e.to_string()))?;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        let endpoint_arc = Arc::new(RwLock::new(endpoint));
        let state = Arc::new(RwLock::new(NodeState::Connected));
        let connections = Arc::new(RwLock::new(HashMap::new()));

        // Spawn maintenance task
        let endpoint_clone = Arc::clone(&endpoint_arc);
        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down Iroh node maintenance task");
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(30)) => {
                        // Periodic maintenance
                        if let Ok(ep) = endpoint_clone.try_read() {
                            // Check connection health
                        }
                    }
                }
            }
        });

        Ok(Arc::new(Self {
            endpoint: endpoint_arc,
            docs_store: Arc::new(RwLock::new(docs_store)),
            shutdown_tx,
            state,
            connections,
        }))
    }

    pub async fn info(&self) -> Result<NodeInfo, WalletError> {
        let endpoint = self.endpoint.read().await;
        let state = self.state.read().await.clone();
        
        Ok(NodeInfo {
            node_id: endpoint.node_id().to_string(),
            relay_addresses: vec![], // Populate from endpoint
            is_connected: matches!(state, NodeState::Connected | NodeState::Syncing),
            stats: ConnectionStats {
                bytes_sent: 0,
                bytes_received: 0,
                active_connections: 0,
                pending_connections: 0,
                total_connections: 0,
            },
            state,
        })
    }

    pub async fn connect(&self, node_addr: String) -> Result<Arc<ConnectionHandle>, WalletError> {
        let endpoint = self.endpoint.read().await;
        let addr: NodeAddr = node_addr.parse()
            .map_err(|e| WalletError::InvalidAddress(e.to_string()))?;
        
        let alpn = b"iroh-wallet/1";
        let conn = endpoint.connect(addr, alpn)
            .await
            .map_err(|e| WalletError::ConnectionFailed(e.to_string()))?;

        let handle = Arc::new(ConnectionHandle {
            id: uuid::Uuid::new_v4().to_string(),
            connection: Arc::new(RwLock::new(conn)),
        });

        // Store connection
        let mut connections = self.connections.write().await;
        connections.insert(handle.id.clone(), 
            handle.connection.read().await.clone());

        Ok(handle)
    }

    pub async fn open_document(&self, namespace_id: String) -> Result<Arc<DocumentHandle>, WalletError> {
        let store = self.docs_store.read().await;
        
        // Parse namespace ID
        let ns_id: NamespaceId = namespace_id.parse()
            .map_err(|e| WalletError::InvalidKey(e.to_string()))?;
        
        // Open or create document
        let doc = store.open(ns_id)
            .map_err(|e| WalletError::DocumentNotFound(e.to_string()))?;

        Ok(Arc::new(DocumentHandle {
            id: namespace_id,
            doc: Arc::new(RwLock::new(doc)),
        }))
    }

    pub async fn shutdown(&self) -> Result<(), WalletError> {
        // Update state
        let mut state = self.state.write().await;
        *state = NodeState::Disconnecting;

        // Send shutdown signal
        let _ = self.shutdown_tx.send(()).await;

        // Close endpoint
        let endpoint = self.endpoint.read().await;
        endpoint.close().await;

        *state = NodeState::Disconnected;
        Ok(())
    }
}

// ============== ConnectionHandle Implementation ==============

#[uniffi::export(async_runtime = "tokio")]
impl ConnectionHandle {
    pub async fn close(&self) -> Result<(), WalletError> {
        let conn = self.connection.read().await;
        conn.close(0u32.into(), b"closing");
        Ok(())
    }
}

// ============== DocumentHandle Implementation ==============

#[uniffi::export(async_runtime = "tokio")]
impl DocumentHandle {
    pub async fn write(&self, key: String, value: Vec<u8>) -> Result<(), WalletError> {
        let doc = self.doc.read().await;
        // Implementation depends on iroh-docs API
        Ok(())
    }

    pub async fn read(&self, key: String) -> Result<Vec<u8>, WalletError> {
        let doc = self.doc.read().await;
        // Implementation depends on iroh-docs API
        Ok(vec![])
    }

    pub async fn sync(&self, options: SyncOptions) -> Result<SyncResult, WalletError> {
        let timeout = Duration::from_secs(options.timeout_secs as u64);
        
        // Perform sync with timeout
        let result = tokio::time::timeout(timeout, async {
            // Sync implementation
            Ok::<_, WalletError>(SyncResult {
                success: true,
                entries_synced: 0,
                entries_failed: 0,
                error_message: None,
            })
        }).await;

        match result {
            Ok(Ok(sync_result)) => Ok(sync_result),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(WalletError::Timeout),
        }
    }
}
```

---

## 3. React Native Module

### 3.1 TypeScript Interface

```typescript
// src/index.ts
import { NativeModules, Platform, EventEmitter, NativeEventEmitter } from 'react-native';

const { IrohWalletModule } = NativeModules;

// Re-export generated types
export interface NodeConfig {
    relayUrls: string[];
    enableDiscovery: boolean;
    alpnProtocol?: string;
    maxConnections: number;
    idleTimeoutSecs: number;
}

export interface NodeInfo {
    nodeId: string;
    relayAddresses: string[];
    isConnected: boolean;
    stats: ConnectionStats;
    state: NodeState;
}

export interface ConnectionStats {
    bytesSent: number;
    bytesReceived: number;
    activeConnections: number;
    pendingConnections: number;
    totalConnections: number;
}

export type NodeState = 
    | 'Initializing' 
    | 'Connecting' 
    | 'Connected' 
    | 'Syncing' 
    | 'Disconnecting' 
    | 'Disconnected' 
    | 'Error';

export interface SyncOptions {
    bidirectional: boolean;
    timeoutSecs: number;
    filterPrefixes?: string[];
}

export interface SyncResult {
    success: boolean;
    entriesSynced: number;
    entriesFailed: number;
    errorMessage?: string;
}

export interface Entry {
    key: string;
    value: Uint8Array;
    author: string;
    timestamp: number;
}

export interface Identity {
    id: string;
    name: string;
    publicKey: string;
    createdAt: number;
}

export interface Signature {
    data: Uint8Array;
    algorithm: string;
    timestamp: number;
}

export type DocumentCallback = {
    onEntryChanged: (key: string, value: Uint8Array, author: string) => void;
    onPeerJoined: (peerId: string) => void;
    onPeerLeft: (peerId: string) => void;
    onSyncStarted: () => void;
    onSyncCompleted: (entriesSynced: number) => void;
    onError: (error: string) => void;
};

// Wallet error class
export class WalletError extends Error {
    constructor(
        message: string,
        public code: string,
        public details?: Record<string, any>
    ) {
        super(message);
        this.name = 'WalletError';
    }
}

/**
 * Main Iroh Wallet API
 */
export class IrohWallet {
    private nodeHandle: any = null;
    private isInitialized: boolean = false;
    private eventEmitter: NativeEventEmitter;
    private eventListeners: Map<string, Function[]> = new Map();

    constructor() {
        this.eventEmitter = new NativeEventEmitter(IrohWalletModule);
        this.setupEventListeners();
    }

    private setupEventListeners() {
        // Listen for native events
        this.eventEmitter.addListener('onPeerJoined', (event) => {
            this.emit('peerJoined', event.peerId);
        });

        this.eventEmitter.addListener('onPeerLeft', (event) => {
            this.emit('peerLeft', event.peerId);
        });

        this.eventEmitter.addListener('onSyncCompleted', (event) => {
            this.emit('syncCompleted', event.entriesSynced);
        });

        this.eventEmitter.addListener('onError', (event) => {
            this.emit('error', new WalletError(event.message, event.code));
        });
    }

    /**
     * Initialize the Iroh node
     */
    async initialize(config: Partial<NodeConfig> = {}): Promise<void> {
        if (this.isInitialized) {
            throw new WalletError('Node already initialized', 'AlreadyInitialized');
        }

        const defaultConfig: NodeConfig = {
            relayUrls: ['https://relay.iroh.network'],
            enableDiscovery: true,
            alpnProtocol: 'iroh-wallet/1',
            maxConnections: 10,
            idleTimeoutSecs: 300,
        };

        const mergedConfig = { ...defaultConfig, ...config };

        try {
            this.nodeHandle = await IrohWalletModule.initNode(mergedConfig);
            this.isInitialized = true;
        } catch (error: any) {
            throw this.wrapError(error);
        }
    }

    /**
     * Get node information
     */
    async getNodeInfo(): Promise<NodeInfo> {
        this.ensureInitialized();
        try {
            return await this.nodeHandle.getInfo();
        } catch (error: any) {
            throw this.wrapError(error);
        }
    }

    /**
     * Connect to a remote peer
     */
    async connect(nodeAddr: string): Promise<Connection> {
        this.ensureInitialized();
        try {
            const connHandle = await this.nodeHandle.connect(nodeAddr);
            return new Connection(connHandle);
        } catch (error: any) {
            throw this.wrapError(error);
        }
    }

    /**
     * Open a document for synchronization
     */
    async openDocument(namespaceId: string): Promise<Document> {
        this.ensureInitialized();
        try {
            const docHandle = await this.nodeHandle.openDocument(namespaceId);
            return new Document(docHandle, this);
        } catch (error: any) {
            throw this.wrapError(error);
        }
    }

    /**
     * Create a new identity
     */
    async createIdentity(name: string): Promise<Identity> {
        this.ensureInitialized();
        try {
            return await this.nodeHandle.createIdentity(name);
        } catch (error: any) {
            throw this.wrapError(error);
        }
    }

    /**
     * Sign data with an identity
     */
    async signData(identityId: string, data: Uint8Array): Promise<Signature> {
        this.ensureInitialized();
        try {
            return await this.nodeHandle.signData(identityId, data);
        } catch (error: any) {
            throw this.wrapError(error);
        }
    }

    /**
     * Verify a signature
     */
    async verifySignature(
        identityId: string, 
        data: Uint8Array, 
        signature: Signature
    ): Promise<boolean> {
        this.ensureInitialized();
        try {
            return await this.nodeHandle.verifySignature(identityId, data, signature);
        } catch (error: any) {
            throw this.wrapError(error);
        }
    }

    /**
     * Gracefully shutdown the node
     */
    async shutdown(): Promise<void> {
        if (!this.isInitialized) return;

        try {
            await this.nodeHandle.shutdown();
            this.nodeHandle = null;
            this.isInitialized = false;
        } catch (error: any) {
            throw this.wrapError(error);
        }
    }

    /**
     * Subscribe to wallet events
     */
    on(event: string, callback: Function): () => void {
        if (!this.eventListeners.has(event)) {
            this.eventListeners.set(event, []);
        }
        this.eventListeners.get(event)!.push(callback);

        return () => {
            const listeners = this.eventListeners.get(event);
            if (listeners) {
                const index = listeners.indexOf(callback);
                if (index > -1) {
                    listeners.splice(index, 1);
                }
            }
        };
    }

    private emit(event: string, ...args: any[]) {
        const listeners = this.eventListeners.get(event);
        if (listeners) {
            listeners.forEach(cb => cb(...args));
        }
    }

    private ensureInitialized(): void {
        if (!this.isInitialized) {
            throw new WalletError('Node not initialized', 'NotInitialized');
        }
    }

    private wrapError(error: any): WalletError {
        if (error.code && error.message) {
            return new WalletError(error.message, error.code, error.details);
        }
        return new WalletError(String(error), 'UnknownError');
    }
}

/**
 * Connection wrapper
 */
export class Connection {
    constructor(private handle: any) {}

    async close(): Promise<void> {
        await this.handle.close();
    }
}

/**
 * Document wrapper
 */
export class Document {
    constructor(private handle: any, private wallet: IrohWallet) {}

    async write(key: string, value: Uint8Array): Promise<void> {
        await this.handle.write(key, value);
    }

    async read(key: string): Promise<Uint8Array> {
        return await this.handle.read(key);
    }

    async getEntries(): Promise<Entry[]> {
        return await this.handle.getEntries();
    }

    async sync(options?: Partial<SyncOptions>): Promise<SyncResult> {
        const defaultOptions: SyncOptions = {
            bidirectional: true,
            timeoutSecs: 30,
        };
        return await this.handle.sync({ ...defaultOptions, ...options });
    }

    subscribe(callbacks: Partial<DocumentCallback>): () => void {
        // Set up subscription
        const subscription = this.handle.subscribe(callbacks);
        
        return () => {
            subscription.unsubscribe();
        };
    }
}

// Singleton instance
let walletInstance: IrohWallet | null = null;

export function getWallet(): IrohWallet {
    if (!walletInstance) {
        walletInstance = new IrohWallet();
    }
    return walletInstance;
}

// React Hook
export function useIrohWallet() {
    const [isReady, setIsReady] = React.useState(false);
    const [nodeInfo, setNodeInfo] = React.useState<NodeInfo | null>(null);
    const [error, setError] = React.useState<WalletError | null>(null);

    React.useEffect(() => {
        const wallet = getWallet();
        
        const init = async () => {
            try {
                await wallet.initialize();
                const info = await wallet.getNodeInfo();
                setNodeInfo(info);
                setIsReady(true);
            } catch (e) {
                setError(e as WalletError);
            }
        };

        init();

        return () => {
            wallet.shutdown();
        };
    }, []);

    return { wallet: getWallet(), isReady, nodeInfo, error };
}
```

---

## 4. iOS Background Execution

### 4.1 AppDelegate.swift

```swift
// ios/AppDelegate.swift
import UIKit
import PushKit
import BackgroundTasks
import React
import ReactNativeIrohWallet

@main
class AppDelegate: RCTAppDelegate, PKPushRegistryDelegate {
    var voipRegistry: PKPushRegistry?
    var backgroundTask: UIBackgroundTaskIdentifier = .invalid
    
    override func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        self.moduleName = "IrohWallet"
        self.initialProps = [:]
        
        // Setup background execution
        setupPushKit()
        setupBackgroundTasks()
        
        return super.application(application, didFinishLaunchingWithOptions: launchOptions)
    }
    
    // MARK: - PushKit Setup
    
    func setupPushKit() {
        voipRegistry = PKPushRegistry(queue: DispatchQueue.main)
        voipRegistry?.delegate = self
        voipRegistry?.desiredPushTypes = [.voIP]
    }
    
    func pushRegistry(
        _ registry: PKPushRegistry,
        didUpdate pushCredentials: PKPushCredentials,
        for type: PKPushType
    ) {
        let token = pushCredentials.token.map { String(format: "%02.2hhx", $0) }.joined()
        IrohWalletModule.shared.setVoipToken(token)
    }
    
    func pushRegistry(
        _ registry: PKPushRegistry,
        didReceiveIncomingPushWith payload: PKPushPayload,
        for type: PKPushType,
        completion: @escaping () -> Void
    ) {
        guard type == .voIP else {
            completion()
            return
        }
        
        let dict = payload.dictionaryPayload
        
        // Handle sign request
        if let signRequest = dict["sign_request"] as? [String: Any] {
            // Begin background task
            backgroundTask = UIApplication.shared.beginBackgroundTask { [weak self] in
                self?.endBackgroundTask()
            }
            
            IrohWalletModule.shared.handleSignRequest(signRequest) { [weak self] result in
                self?.endBackgroundTask()
                completion()
            }
        } else {
            completion()
        }
    }
    
    private func endBackgroundTask() {
        if backgroundTask != .invalid {
            UIApplication.shared.endBackgroundTask(backgroundTask)
            backgroundTask = .invalid
        }
    }
    
    // MARK: - Background Tasks
    
    func setupBackgroundTasks() {
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "com.yourapp.iroh-sync",
            using: nil
        ) { [weak self] task in
            self?.handleSyncTask(task as! BGProcessingTask)
        }
    }
    
    func handleSyncTask(_ task: BGProcessingTask) {
        let queue = OperationQueue()
        queue.maxConcurrentOperationCount = 1
        
        let syncOperation = BlockOperation { [weak self] in
            let semaphore = DispatchSemaphore(value: 0)
            
            IrohWalletModule.shared.performBackgroundSync { success in
                semaphore.signal()
            }
            
            semaphore.wait()
        }
        
        task.expirationHandler = {
            queue.cancelAllOperations()
        }
        
        syncOperation.completionBlock = {
            task.setTaskCompleted(success: !syncOperation.isCancelled)
        }
        
        queue.addOperations([syncOperation], waitUntilFinished: false)
    }
    
    // MARK: - Silent Push Notifications
    
    override func application(
        _ application: UIApplication,
        didReceiveRemoteNotification userInfo: [AnyHashable: Any],
        fetchCompletionHandler completionHandler: @escaping (UIBackgroundFetchResult) -> Void
    ) {
        if let syncRequest = userInfo["sync"] as? [String: Any] {
            IrohWalletModule.shared.handleSyncRequest(syncRequest) { result in
                completionHandler(result ? .newData : .noData)
            }
        } else {
            completionHandler(.noData)
        }
    }
}
```

### 4.2 Info.plist

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Background modes -->
    <key>UIBackgroundModes</key>
    <array>
        <string>voip</string>
        <string>fetch</string>
        <string>remote-notification</string>
        <string>processing</string>
    </array>
    
    <!-- PushKit -->
    <key>PKPushType</key>
    <array>
        <string>PKPushTypeVoIP</string>
    </array>
    
    <!-- Background task identifiers -->
    <key>BGTaskSchedulerPermittedIdentifiers</key>
    <array>
        <string>com.yourapp.iroh-sync</string>
    </array>
</dict>
</plist>
```

---

## 5. Android Background Execution

### 5.1 AndroidManifest.xml

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.yourapp.irohwallet">

    <!-- Network permissions -->
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    
    <!-- Foreground service -->
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC" />
    
    <!-- Wake lock -->
    <uses-permission android:name="android.permission.WAKE_LOCK" />
    
    <!-- Push notifications -->
    <uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
    
    <!-- Ignore battery optimizations (request at runtime) -->
    <uses-permission android:name="android.permission.REQUEST_IGNORE_BATTERY_OPTIMIZATIONS" />

    <application
        android:name=".MainApplication"
        android:allowBackup="true"
        android:icon="@mipmap/ic_launcher"
        android:label="@string/app_name"
        android:theme="@style/AppTheme">
        
        <activity android:name=".MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
        
        <!-- Iroh P2P Service -->
        <service
            android:name=".IrohP2PService"
            android:enabled="true"
            android:exported="false"
            android:foregroundServiceType="dataSync" />
        
        <!-- Firebase Messaging Service -->
        <service
            android:name=".IrohMessagingService"
            android:exported="false">
            <intent-filter>
                <action android:name="com.google.firebase.MESSAGING_EVENT" />
            </intent-filter>
        </service>
        
        <!-- Boot receiver -->
        <receiver
            android:name=".BootReceiver"
            android:enabled="true"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.BOOT_COMPLETED" />
            </intent-filter>
        </receiver>
        
    </application>
</manifest>
```

### 5.2 IrohP2PService.kt

```kotlin
package com.yourapp.irohwallet

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.*

class IrohP2PService : Service() {
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var wakeLock: PowerManager.WakeLock? = null
    private val NOTIFICATION_ID = 1001
    private val CHANNEL_ID = "iroh_p2p_channel"
    
    companion object {
        const val ACTION_START = "com.yourapp.START_P2P"
        const val ACTION_STOP = "com.yourapp.STOP_P2P"
        const val ACTION_SYNC = "com.yourapp.SYNC"
        
        fun start(context: Context) {
            val intent = Intent(context, IrohP2PService::class.java).apply {
                action = ACTION_START
            }
            ContextCompat.startForegroundService(context, intent)
        }
        
        fun stop(context: Context) {
            val intent = Intent(context, IrohP2PService::class.java).apply {
                action = ACTION_STOP
            }
            context.startService(intent)
        }
    }
    
    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }
    
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> startP2P()
            ACTION_STOP -> stopP2P()
            ACTION_SYNC -> triggerSync()
        }
        return START_STICKY
    }
    
    private fun startP2P() {
        // Acquire wake lock
        val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = powerManager.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "IrohWallet::P2PWakelock"
        ).apply {
            acquire(10 * 60 * 1000L)
        }
        
        // Start as foreground service
        startForeground(NOTIFICATION_ID, createNotification())
        
        // Initialize Iroh
        serviceScope.launch {
            try {
                IrohWalletModule.initializeNode()
                maintainConnections()
            } catch (e: Exception) {
                // Handle error
            }
        }
    }
    
    private fun stopP2P() {
        serviceScope.launch {
            IrohWalletModule.shutdownNode()
            wakeLock?.release()
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        }
    }
    
    private suspend fun maintainConnections() {
        while (isActive) {
            delay(30000)
            IrohWalletModule.maintainConnections()
        }
    }
    
    private fun triggerSync() {
        serviceScope.launch {
            IrohWalletModule.performSync()
        }
    }
    
    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Iroh P2P Network",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Maintains P2P connections"
                setShowBadge(false)
            }
            getSystemService(NotificationManager::class.java)
                .createNotificationChannel(channel)
        }
    }
    
    private fun createNotification(): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this, 0,
            packageManager.getLaunchIntentForPackage(packageName),
            PendingIntent.FLAG_IMMUTABLE
        )
        
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Identity Wallet")
            .setContentText("P2P network active")
            .setSmallIcon(R.drawable.ic_notification)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setSilent(true)
            .build()
    }
    
    override fun onBind(intent: Intent?): IBinder? = null
    
    override fun onDestroy() {
        super.onDestroy()
        serviceScope.cancel()
        wakeLock?.release()
    }
}
```

### 5.3 IrohMessagingService.kt

```kotlin
package com.yourapp.irohwallet

import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch

class IrohMessagingService : FirebaseMessagingService() {
    
    override fun onMessageReceived(message: RemoteMessage) {
        super.onMessageReceived(message)
        
        when (message.data["type"]) {
            "sign_request" -> handleSignRequest(message.data)
            "sync_trigger" -> handleSyncTrigger()
            "relay_update" -> handleRelayUpdate(message.data)
        }
    }
    
    override fun onNewToken(token: String) {
        super.onNewToken(token)
        IrohWalletModule.updateFcmToken(token)
    }
    
    private fun handleSignRequest(data: Map<String, String>) {
        val requestId = data["request_id"] ?: return
        val documentId = data["document_id"] ?: return
        
        // Start service
        IrohP2PService.start(this)
        
        GlobalScope.launch {
            IrohWalletModule.handleSignRequest(requestId, documentId)
        }
    }
    
    private fun handleSyncTrigger() {
        IrohSyncWorker.scheduleImmediate(this)
    }
    
    private fun handleRelayUpdate(data: Map<String, String>) {
        val relayUrls = data["relays"]?.split(",") ?: return
        IrohWalletModule.updateRelayServers(relayUrls)
    }
}
```

### 5.4 IrohSyncWorker.kt

```kotlin
package com.yourapp.irohwallet

import android.content.Context
import androidx.work.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class IrohSyncWorker(
    context: Context,
    params: WorkerParameters
) : CoroutineWorker(context, params) {
    
    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        try {
            val syncResult = IrohWalletModule.performBackgroundSync()
            if (syncResult) Result.success() else Result.retry()
        } catch (e: Exception) {
            Result.failure()
        }
    }
    
    companion object {
        private const val WORK_NAME = "iroh_sync_work"
        
        fun schedule(context: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()
            
            val syncWork = PeriodicWorkRequestBuilder<IrohSyncWorker>(
                15, java.util.concurrent.TimeUnit.MINUTES
            )
                .setConstraints(constraints)
                .build()
            
            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                WORK_NAME,
                ExistingPeriodicWorkPolicy.KEEP,
                syncWork
            )
        }
        
        fun scheduleImmediate(context: Context) {
            val syncWork = OneTimeWorkRequestBuilder<IrohSyncWorker>()
                .setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
                .build()
            
            WorkManager.getInstance(context).enqueue(syncWork)
        }
    }
}
```

---

## 6. Push Notification Server

### 6.1 Node.js Push Server

```javascript
// server/push-server.js
const express = require('express');
const admin = require('firebase-admin');
const apn = require('apn');

// Initialize Firebase Admin
admin.initializeApp({
    credential: admin.credential.cert(require('./service-account.json'))
});

// Initialize APN for iOS
const apnProvider = new apn.Provider({
    token: {
        key: './apns-key.p8',
        keyId: 'YOUR_KEY_ID',
        teamId: 'YOUR_TEAM_ID'
    },
    production: false
});

const app = express();
app.use(express.json());

// Store device tokens (use a database in production)
const deviceTokens = new Map();

/**
 * Register device token
 */
app.post('/register', (req, res) => {
    const { userId, fcmToken, voipToken, platform } = req.body;
    
    deviceTokens.set(userId, {
        fcmToken,
        voipToken,
        platform,
        registeredAt: Date.now()
    });
    
    res.json({ success: true });
});

/**
 * Send sign request to device (Wake-and-Sign)
 */
app.post('/sign-request', async (req, res) => {
    const { userId, requestId, documentId, key, timeout = 60 } = req.body;
    
    const device = deviceTokens.get(userId);
    if (!device) {
        return res.status(404).json({ error: 'Device not found' });
    }
    
    try {
        if (device.platform === 'ios') {
            await sendIosSignRequest(device.voipToken, {
                requestId,
                documentId,
                key,
                timeout
            });
        } else {
            await sendAndroidSignRequest(device.fcmToken, {
                requestId,
                documentId,
                key,
                timeout
            });
        }
        
        res.json({ success: true, message: 'Sign request sent' });
    } catch (error) {
        console.error('Failed to send sign request:', error);
        res.status(500).json({ error: 'Failed to send sign request' });
    }
});

/**
 * Send sync trigger to device
 */
app.post('/sync-trigger', async (req, res) => {
    const { userId, documentId } = req.body;
    
    const device = deviceTokens.get(userId);
    if (!device) {
        return res.status(404).json({ error: 'Device not found' });
    }
    
    try {
        if (device.platform === 'ios') {
            await sendIosSilentPush(device.fcmToken, {
                sync: { documentId }
            });
        } else {
            await sendAndroidDataMessage(device.fcmToken, {
                type: 'sync_trigger',
                documentId
            });
        }
        
        res.json({ success: true });
    } catch (error) {
        console.error('Failed to send sync trigger:', error);
        res.status(500).json({ error: 'Failed to send sync trigger' });
    }
});

/**
 * Send iOS VoIP push for sign request
 */
async function sendIosSignRequest(voipToken, data) {
    const notification = new apn.Notification({
        topic: 'com.yourapp.irohwallet.voip',
        pushType: 'voip',
        priority: 10
    });
    
    notification.payload = {
        sign_request: data
    };
    
    const result = await apnProvider.send(notification, voipToken);
    
    if (result.failed.length > 0) {
        throw new Error(`APNs error: ${result.failed[0].response}`);
    }
}

/**
 * Send iOS silent push
 */
async function sendIosSilentPush(deviceToken, data) {
    const notification = new apn.Notification({
        topic: 'com.yourapp.irohwallet',
        contentAvailable: true,
        priority: 5,
        pushType: 'background'
    });
    
    notification.payload = data;
    
    const result = await apnProvider.send(notification, deviceToken);
    
    if (result.failed.length > 0) {
        throw new Error(`APNs error: ${result.failed[0].response}`);
    }
}

/**
 * Send Android high-priority data message
 */
async function sendAndroidSignRequest(fcmToken, data) {
    const message = {
        token: fcmToken,
        data: {
            type: 'sign_request',
            request_id: data.requestId,
            document_id: data.documentId,
            key: data.key,
            timeout: String(data.timeout)
        },
        android: {
            priority: 'high',
            directBootOk: true
        }
    };
    
    await admin.messaging().send(message);
}

/**
 * Send Android data message
 */
async function sendAndroidDataMessage(fcmToken, data) {
    const message = {
        token: fcmToken,
        data: {
            type: data.type,
            document_id: data.documentId
        },
        android: {
            priority: 'normal'
        }
    };
    
    await admin.messaging().send(message);
}

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => {
    console.log(`Push server listening on port ${PORT}`);
});
```

---

*Code examples for Iroh Mobile Integration*
