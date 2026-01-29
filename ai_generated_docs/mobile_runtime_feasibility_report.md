# Mobile Runtime Feasibility Report: Iroh Rust Crate Integration

## Executive Summary

This report analyzes the feasibility of running the Iroh Rust crate in a cross-platform mobile environment for a Local-First identity wallet. The analysis covers UniFFi integration for React Native, background execution patterns for P2P networking, cloud relay integration for "Wake-and-Sign" patterns, and battery optimization strategies.

**Key Findings:**
- ✅ **UniFFi for React Native** is production-ready and actively maintained by Mozilla/Filament
- ✅ **Iroh FFI bindings** exist for Swift/Kotlin but React Native requires custom UniFFi integration
- ⚠️ **Background execution** has significant platform constraints requiring push notification triggers
- ✅ **Cloud Relay integration** is a core Iroh feature (DERP protocol)
- ⚠️ **Battery optimization** requires careful wake-connect-sync-sleep patterns

---

## 1. UniFFi Integration for React Native

### 1.1 Overview

Mozilla's `uniffi-bindgen-react-native` (abbreviated as `ubrn`) provides a mature solution for exposing Rust APIs to React Native through Turbo Modules. This tool generates:

- TypeScript bindings that mirror Rust definitions
- JSI C++ code for JS-to-Rust communication
- TurboModule infrastructure for both iOS and Android

**Citations:**
- [Mozilla UniFFi React Native Tutorial](https://jhugman.github.io/uniffi-bindgen-react-native/guides/rn/getting-started.html) [^3^]
- [Mozilla Hacks: Introducing UniFFi for React Native](https://hacks.mozilla.org/2024/12/introducing-uniffi-for-react-native-rust-powered-turbo-modules/) [^54^]
- [UniFFi User Guide - Async Support](https://mozilla.github.io/uniffi-rs/0.28/futures.html) [^5^]

### 1.2 Project Setup

```bash
# Step 1: Create React Native library with Turbo Module support
npx create-react-native-library@latest my-iroh-wallet
# Select: Turbo module, C++ for Android & iOS

# Step 2: Add uniffi-bindgen-react-native
cd my-iroh-wallet
yarn add uniffi-bindgen-react-native

# Step 3: Create configuration file
```

### 1.3 ubrn.config.yaml Configuration

```yaml
---
# ubrn.config.yaml - UniFFi Bindgen React Native Configuration
rust:
  repo: https://github.com/n0-computer/iroh.git
  branch: main
  manifestPath: iroh/Cargo.toml
  
# Alternative: Local crate path
# rust:
#   crate: ../iroh-wallet-core

generate:
  # Output directory for generated bindings
  output: src/generated
  
# iOS specific
ios:
  targets:
    - aarch64-apple-ios
    - aarch64-apple-ios-sim
    - x86_64-apple-ios
    
# Android specific  
android:
  targets:
    - aarch64-linux-android
    - armv7-linux-androideabi
    - x86_64-linux-android
```

### 1.4 UniFFi UDL Interface Definition for Iroh

Create `src/iroh_wallet.udl`:

```idl
// Iroh Wallet UniFFi Interface Definition
namespace iroh_wallet {
    // Initialize the Iroh node with configuration
    [Async]
    NodeHandle init_node(NodeConfig config);
    
    // Get node status and connection info
    NodeInfo get_node_info(NodeHandle handle);
    
    // Shutdown the node gracefully
    [Async]
    void shutdown_node(NodeHandle handle);
};

// Node configuration
dictionary NodeConfig {
    sequence<string> relay_urls;
    boolean enable_discovery;
    string? alpn_protocol;
};

// Node information returned after initialization
dictionary NodeInfo {
    string node_id;
    sequence<string> relay_addresses;
    boolean is_connected;
    ConnectionStats stats;
};

// Connection statistics
dictionary ConnectionStats {
    u64 bytes_sent;
    u64 bytes_received;
    u32 active_connections;
    u32 pending_connections;
};

// Document synchronization interface
interface DocumentSync {
    constructor();
    
    // Open a document for syncing
    [Async]
    DocumentHandle open_document(string document_id);
    
    // Subscribe to document changes
    [Async]
    SubscriptionHandle subscribe(DocumentHandle doc, DocumentCallback callback);
    
    // Write data to document
    [Async]
    void write(DocumentHandle doc, string key, sequence<u8> value);
    
    // Read data from document
    [Async]
    sequence<u8> read(DocumentHandle doc, string key);
};

// Callback interface for document changes
callback interface DocumentCallback {
    [Async]
    void on_change(string key, sequence<u8> value);
    
    [Async]
    void on_peer_join(string peer_id);
    
    [Async]
    void on_peer_leave(string peer_id);
};

// Handle types (opaque pointers)
interface NodeHandle {};
interface DocumentHandle {};
interface SubscriptionHandle {};

// Error definitions
[Error]
enum WalletError {
    "NodeInitFailed",
    "ConnectionFailed",
    "DocumentNotFound",
    "SyncFailed",
    "InvalidKey",
    "NetworkError",
    "Timeout"
};
```

### 1.5 Rust Implementation with Proc Macros

Alternative to UDL: Use UniFFi proc macros directly in Rust:

```rust
// src/lib.rs - Iroh Wallet Core Rust Implementation

use std::sync::Arc;
use iroh::{Endpoint, RelayMode, NodeAddr};
use iroh_docs::{Doc, Author, NamespaceId};
use tokio::sync::RwLock;

uniffi::setup_scaffolding!();

/// Node configuration for initialization
#[derive(uniffi::Record, Clone, Debug)]
pub struct NodeConfig {
    pub relay_urls: Vec<String>,
    pub enable_discovery: bool,
    pub alpn_protocol: Option<String>,
}

/// Node information returned after initialization
#[derive(uniffi::Record, Clone, Debug)]
pub struct NodeInfo {
    pub node_id: String,
    pub relay_addresses: Vec<String>,
    pub is_connected: bool,
    pub stats: ConnectionStats,
}

#[derive(uniffi::Record, Clone, Debug)]
pub struct ConnectionStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active_connections: u32,
    pub pending_connections: u32,
}

/// Opaque handle to an Iroh node
#[derive(uniffi::Object)]
pub struct NodeHandle {
    endpoint: Arc<RwLock<Endpoint>>,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
}

#[uniffi::export(async_runtime = "tokio")]
impl NodeHandle {
    /// Initialize a new Iroh node with the given configuration
    #[uniffi::constructor]
    pub async fn new(config: NodeConfig) -> Result<Arc<Self>, WalletError> {
        let relay_mode = if config.relay_urls.is_empty() {
            RelayMode::Disabled
        } else {
            RelayMode::Custom(
                config.relay_urls.iter()
                    .filter_map(|url| url.parse().ok())
                    .collect()
            )
        };

        let endpoint = Endpoint::builder()
            .relay_mode(relay_mode)
            .discovery(config.enable_discovery)
            .bind()
            .await
            .map_err(|e| WalletError::NodeInitFailed(e.to_string()))?;

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);
        let endpoint_arc = Arc::new(RwLock::new(endpoint));

        // Spawn background task for connection maintenance
        let endpoint_clone = Arc::clone(&endpoint_arc);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                        // Periodic maintenance: check connections, relay health
                        if let Ok(ep) = endpoint_clone.try_read() {
                            // Connection maintenance logic
                        }
                    }
                }
            }
        });

        Ok(Arc::new(Self {
            endpoint: endpoint_arc,
            shutdown_tx,
        }))
    }

    /// Get current node information
    pub async fn info(&self) -> Result<NodeInfo, WalletError> {
        let endpoint = self.endpoint.read().await;
        let node_id = endpoint.node_id().to_string();
        
        Ok(NodeInfo {
            node_id,
            relay_addresses: vec![], // Populate from endpoint
            is_connected: true,
            stats: ConnectionStats {
                bytes_sent: 0,
                bytes_received: 0,
                active_connections: 0,
                pending_connections: 0,
            },
        })
    }

    /// Connect to a remote node
    pub async fn connect(&self, node_addr: String) -> Result<ConnectionHandle, WalletError> {
        let endpoint = self.endpoint.read().await;
        let addr: NodeAddr = node_addr.parse()
            .map_err(|e| WalletError::InvalidAddress(e.to_string()))?;
        
        // Establish connection
        let conn = endpoint.connect(addr, b"iroh-wallet/1")
            .await
            .map_err(|e| WalletError::ConnectionFailed(e.to_string()))?;

        Ok(ConnectionHandle { inner: conn })
    }

    /// Gracefully shutdown the node
    pub async fn shutdown(&self) -> Result<(), WalletError> {
        let _ = self.shutdown_tx.send(()).await;
        let endpoint = self.endpoint.read().await;
        endpoint.close().await;
        Ok(())
    }
}

/// Opaque connection handle
#[derive(uniffi::Object)]
pub struct ConnectionHandle {
    inner: iroh::Connection,
}

/// Document synchronization for iroh-docs
#[derive(uniffi::Object)]
pub struct DocumentSync {
    docs: Arc<RwLock<HashMap<String, Doc>>>,
}

#[uniffi::export(async_runtime = "tokio")]
impl DocumentSync {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            docs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Open a document for synchronization
    pub async fn open(&self, document_id: String) -> Result<DocumentHandle, WalletError> {
        // Implementation using iroh-docs
        Ok(DocumentHandle { id: document_id })
    }
}

#[derive(uniffi::Object)]
pub struct DocumentHandle {
    id: String,
}

/// Error types for the wallet
#[derive(uniffi::Error, Debug)]
pub enum WalletError {
    NodeInitFailed(String),
    ConnectionFailed(String),
    DocumentNotFound(String),
    SyncFailed(String),
    InvalidKey(String),
    NetworkError(String),
    Timeout,
    InvalidAddress(String),
}

impl std::fmt::Display for WalletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for WalletError {}
```

### 1.6 Async Handling Across FFI Boundary

UniFFi supports async functions with the `async_runtime = "tokio"` attribute:

```rust
// Async function export with Tokio runtime
#[uniffi::export(async_runtime = "tokio")]
pub async fn sync_document(doc_id: String) -> Result<SyncResult, WalletError> {
    // This runs on the Tokio runtime
    let result = perform_sync(doc_id).await?;
    Ok(result)
}
```

**Key considerations for async FFI:**
- The foreign language (TypeScript) provides the executor/event loop
- Rust futures are polled by the foreign runtime
- Cancellation is supported but requires careful handling
- Use `tokio::sync` primitives for cross-boundary communication

### 1.7 Error Propagation Strategies

```rust
// Comprehensive error handling with context
#[derive(uniffi::Error, Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Node initialization failed: {0}")]
    NodeInitFailed(String),
    
    #[error("Connection failed to {node_id}: {reason}")]
    ConnectionFailed { node_id: String, reason: String },
    
    #[error("Document not found: {0}")]
    DocumentNotFound(String),
    
    #[error("Sync operation failed: {0}")]
    SyncFailed(#[from] iroh_docs::Error),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] iroh::EndpointError),
    
    #[error("Operation timed out after {0}ms")]
    Timeout(u64),
}
```

---

## 2. React Native Wrapper Architecture

### 2.1 Module Structure

```typescript
// src/index.ts - React Native Module Entry Point
import { NativeModules, Platform } from 'react-native';
import type { NodeConfig, NodeInfo, WalletError } from './generated/iroh_wallet';

// Import generated UniFFi bindings
const { IrohWalletModule } = NativeModules;

// Re-export generated types
export * from './generated/iroh_wallet';

/**
 * Iroh Wallet API for React Native
 * 
 * This class wraps the UniFFi-generated bindings with a more ergonomic
 * JavaScript/TypeScript interface.
 */
export class IrohWallet {
    private nodeHandle: any = null;
    private isInitialized: boolean = false;
    private eventListeners: Map<string, Set<Function>> = new Map();

    /**
     * Initialize the Iroh node with configuration
     */
    async initialize(config: NodeConfig): Promise<void> {
        if (this.isInitialized) {
            throw new Error('Node already initialized');
        }

        try {
            this.nodeHandle = await IrohWalletModule.init_node(config);
            this.isInitialized = true;
            this.setupEventListeners();
        } catch (error) {
            throw this.wrapError(error);
        }
    }

    /**
     * Get node information and connection status
     */
    async getNodeInfo(): Promise<NodeInfo> {
        this.ensureInitialized();
        try {
            return await this.nodeHandle.info();
        } catch (error) {
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
        } catch (error) {
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
        } catch (error) {
            throw this.wrapError(error);
        }
    }

    /**
     * Subscribe to wallet events
     */
    on(event: string, callback: Function): () => void {
        if (!this.eventListeners.has(event)) {
            this.eventListeners.set(event, new Set());
        }
        this.eventListeners.get(event)!.add(callback);

        // Return unsubscribe function
        return () => {
            this.eventListeners.get(event)?.delete(callback);
        };
    }

    private ensureInitialized(): void {
        if (!this.isInitialized) {
            throw new Error('Node not initialized. Call initialize() first.');
        }
    }

    private setupEventListeners(): void {
        // Set up native event listeners
    }

    private wrapError(error: any): Error {
        // Convert native errors to JavaScript errors
        if (error.code && error.message) {
            const walletError = new Error(error.message);
            (walletError as any).code = error.code;
            return walletError;
        }
        return error instanceof Error ? error : new Error(String(error));
    }
}

/**
 * Connection wrapper for peer connections
 */
export class Connection {
    constructor(private handle: any) {}

    async send(data: Uint8Array): Promise<void> {
        await this.handle.send(data);
    }

    async receive(): Promise<Uint8Array> {
        return await this.handle.receive();
    }

    async close(): Promise<void> {
        await this.handle.close();
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

// Hook for React components
export function useIrohWallet() {
    const [isReady, setIsReady] = React.useState(false);
    const [nodeInfo, setNodeInfo] = React.useState<NodeInfo | null>(null);

    React.useEffect(() => {
        const wallet = getWallet();
        
        const init = async () => {
            // Initialize with default config
            await wallet.initialize({
                relay_urls: [
                    'https://relay.iroh.network',
                    'https://euw1-1.derp.iroh.network'
                ],
                enable_discovery: true,
                alpn_protocol: 'iroh-wallet/1'
            });
            
            const info = await wallet.getNodeInfo();
            setNodeInfo(info);
            setIsReady(true);
        };

        init();

        return () => {
            wallet.shutdown();
        };
    }, []);

    return { wallet: getWallet(), isReady, nodeInfo };
}
```

### 2.2 Package.json Scripts

```json
{
  "name": "react-native-iroh-wallet",
  "version": "0.1.0",
  "scripts": {
    "ubrn:ios": "ubrn build ios --and-generate && (cd example/ios && pod install)",
    "ubrn:android": "ubrn build android --and-generate",
    "ubrn:web": "ubrn build web",
    "ubrn:checkout": "ubrn checkout",
    "ubrn:clean": "rm -rfv cpp/ android/CMakeLists.txt android/src/main/java android/*.cpp ios/ src/Native* src/index.*ts* src/generated/",
    "build": "tsc",
    "test": "jest",
    "lint": "eslint \"**/*.{js,ts,tsx}\""
  },
  "dependencies": {
    "uniffi-bindgen-react-native": "^0.29.0"
  },
  "peerDependencies": {
    "react": "*",
    "react-native": "*"
  }
}
```

---

## 3. Background Execution Analysis

### 3.1 iOS Background Modes

**Citation:** [Apple Developer Forums - iOS Background Execution Limits](https://developer.apple.com/forums/thread/685525) [^17^]

iOS provides several background execution modes relevant to P2P networking:

| Background Mode | Use Case | P2P Applicability |
|-----------------|----------|-------------------|
| `voip` | Voice over IP calls | ✅ Best for "Wake-and-Sign" - wakes app instantly |
| `fetch` | Periodic background fetch | ⚠️ Limited - not guaranteed, throttled |
| `remote-notification` | Silent push notifications | ✅ Good for triggering P2P sync |
| `processing` | Long-running tasks (BGProcessingTask) | ✅ For batch sync operations |
| `audio` | Audio playback | ❌ Not applicable |
| `location` | Location updates | ⚠️ Only if location-based P2P |

#### 3.1.1 Info.plist Configuration

```xml
<!-- ios/Info.plist -->
<key>UIBackgroundModes</key>
<array>
    <!-- For instant wake on sign requests -->
    <string>voip</string>
    <!-- For background sync -->
    <string>fetch</string>
    <!-- For silent push notifications -->
    <string>remote-notification</string>
    <!-- For extended sync operations -->
    <string>processing</string>
</array>

<!-- PushKit configuration for VoIP -->
<key>PKPushType</key>
<array>
    <string>PKPushTypeVoIP</string>
</array>
```

#### 3.1.2 AppDelegate.swift - PushKit Implementation

```swift
// ios/AppDelegate.swift
import UIKit
import PushKit
import React
import ReactNativeIrohWallet

@main
class AppDelegate: RCTAppDelegate, PKPushRegistryDelegate {
    var voipRegistry: PKPushRegistry?
    
    override func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        // Initialize React Native
        self.moduleName = "IrohWallet"
        self.initialProps = [:]
        
        // Setup PushKit for VoIP push notifications
        setupPushKit()
        
        return super.application(application, didFinishLaunchingWithOptions: launchOptions)
    }
    
    // MARK: - PushKit Setup
    
    func setupPushKit() {
        voipRegistry = PKPushRegistry(queue: DispatchQueue.main)
        voipRegistry?.delegate = self
        voipRegistry?.desiredPushTypes = [.voIP]
    }
    
    // MARK: - PKPushRegistryDelegate
    
    func pushRegistry(
        _ registry: PKPushRegistry,
        didUpdate pushCredentials: PKPushCredentials,
        for type: PKPushType
    ) {
        // Send token to your server
        let token = pushCredentials.token.map { String(format: "%02.2hhx", $0) }.joined()
        print("VoIP token: \(token)")
        
        // Forward to React Native module
        IrohWalletModule.shared.setVoipToken(token)
    }
    
    func pushRegistry(
        _ registry: PKPushRegistry,
        didReceiveIncomingPushWith payload: PKPushPayload,
        for type: PKPushType,
        completion: @escaping () -> Void
    ) {
        // Handle incoming VoIP push - this wakes the app instantly
        if type == .voIP {
            let dict = payload.dictionaryPayload
            
            // Check if this is a sign request
            if let signRequest = dict["sign_request"] as? [String: Any] {
                // Wake the Iroh node and process sign request
                IrohWalletModule.shared.handleSignRequest(signRequest) { result in
                    completion()
                }
            } else {
                completion()
            }
        }
    }
    
    // MARK: - Silent Push Notifications
    
    override func application(
        _ application: UIApplication,
        didReceiveRemoteNotification userInfo: [AnyHashable: Any],
        fetchCompletionHandler completionHandler: @escaping (UIBackgroundFetchResult) -> Void
    ) {
        // Handle silent push for P2P sync
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

#### 3.1.3 BGTaskScheduler for Background Processing

```swift
// ios/BackgroundSyncTask.swift
import BackgroundTasks

class BackgroundSyncTask {
    static let taskIdentifier = "com.yourapp.iroh-sync"
    
    static func register() {
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: taskIdentifier,
            using: nil
        ) { task in
            handleSyncTask(task as! BGProcessingTask)
        }
    }
    
    static func schedule() {
        let request = BGProcessingTaskRequest(identifier: taskIdentifier)
        request.requiresNetworkConnectivity = true
        request.requiresExternalPower = false
        
        do {
            try BGTaskScheduler.shared.submit(request)
        } catch {
            print("Failed to schedule sync: \(error)")
        }
    }
    
    static func handleSyncTask(_ task: BGProcessingTask) {
        let queue = OperationQueue()
        queue.maxConcurrentOperationCount = 1
        
        let syncOperation = BlockOperation {
            // Perform P2P sync
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
}
```

### 3.2 Android Background Execution

**Citation:** [Android Background Execution Guide](https://android-developers.googleblog.com/2018/10/modern-background-execution-in-android.html) [^11^]

Android's background execution restrictions (Doze mode, App Standby) significantly impact P2P networking:

| App State | Network Access | P2P Impact |
|-----------|---------------|------------|
| Foreground | Full | ✅ Full P2P functionality |
| Foreground Service | Full with notification | ✅ Reliable connections |
| Background | Limited | ⚠️ Connections may drop |
| Doze | Suspended | ❌ All connections dropped |
| App Standby | Severely throttled | ❌ No P2P activity |

#### 3.2.1 AndroidManifest.xml Configuration

```xml
<!-- android/app/src/main/AndroidManifest.xml -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    
    <!-- Network permissions -->
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    
    <!-- Foreground service permission -->
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC" />
    
    <!-- Wake lock for keeping CPU awake during sync -->
    <uses-permission android:name="android.permission.WAKE_LOCK" />
    
    <!-- Push notification permissions -->
    <uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />
    
    <!-- For Android 13+ notification permission -->
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
    
    <application>
        <!-- Iroh P2P Service -->
        <service
            android:name=".IrohP2PService"
            android:enabled="true"
            android:exported="false"
            android:foregroundServiceType="dataSync" />
        
        <!-- Firebase Messaging Service for push notifications -->
        <service
            android:name=".IrohMessagingService"
            android:exported="false">
            <intent-filter>
                <action android:name="com.google.firebase.MESSAGING_EVENT" />
            </intent-filter>
        </service>
        
        <!-- WorkManager for scheduled sync -->
        <provider
            android:name="androidx.work.impl.WorkManagerInitializer"
            android:authorities="${applicationId}.workmanager-init"
            android:exported="false" />
            
        <!-- Boot receiver for restarting service -->
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

#### 3.2.2 Foreground Service Implementation

```kotlin
// android/app/src/main/java/com/yourapp/IrohP2PService.kt
package com.yourapp

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
import androidx.work.*
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
            
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
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
        // Acquire wake lock for P2P operations
        val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = powerManager.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "IrohWallet::P2PWakelock"
        ).apply {
            acquire(10 * 60 * 1000L) // 10 minutes max
        }
        
        // Start as foreground service
        startForeground(NOTIFICATION_ID, createNotification())
        
        // Initialize Iroh node
        serviceScope.launch {
            try {
                IrohWalletModule.initializeNode()
                // Maintain connections
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
            delay(30000) // 30 seconds
            
            // Check and maintain P2P connections
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
                description = "Maintains P2P connections for identity wallet"
                setShowBadge(false)
            }
            
            val notificationManager = getSystemService(NotificationManager::class.java)
            notificationManager.createNotificationChannel(channel)
        }
    }
    
    private fun createNotification(): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
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

#### 3.2.3 WorkManager for Scheduled Sync

```kotlin
// android/app/src/main/java/com/yourapp/IrohSyncWorker.kt
package com.yourapp

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
            // Perform P2P sync
            val syncResult = IrohWalletModule.performBackgroundSync()
            
            if (syncResult) {
                Result.success()
            } else {
                Result.retry()
            }
        } catch (e: Exception) {
            Result.failure()
        }
    }
    
    companion object {
        private const val WORK_NAME = "iroh_sync_work"
        
        fun schedule(context: Context) {
            // Constraints: require network
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()
            
            // Periodic work: every 15 minutes (minimum)
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

#### 3.2.4 Firebase Messaging for Push Triggers

```kotlin
// android/app/src/main/java/com/yourapp/IrohMessagingService.kt
package com.yourapp

import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch

class IrohMessagingService : FirebaseMessagingService() {
    
    override fun onMessageReceived(message: RemoteMessage) {
        super.onMessageReceived(message)
        
        val data = message.data
        
        when (data["type"]) {
            "sign_request" -> handleSignRequest(data)
            "sync_trigger" -> handleSyncTrigger(data)
            "relay_update" -> handleRelayUpdate(data)
        }
    }
    
    override fun onNewToken(token: String) {
        super.onNewToken(token)
        // Send token to your server
        IrohWalletModule.updateFcmToken(token)
    }
    
    private fun handleSignRequest(data: Map<String, String>) {
        // High priority - wake app if needed
        val requestId = data["request_id"] ?: return
        val documentId = data["document_id"] ?: return
        
        // Start foreground service if not running
        IrohP2PService.start(this)
        
        // Process sign request
        GlobalScope.launch {
            IrohWalletModule.handleSignRequest(requestId, documentId)
        }
    }
    
    private fun handleSyncTrigger(data: Map<String, String>) {
        // Schedule immediate sync
        IrohSyncWorker.scheduleImmediate(this)
    }
    
    private fun handleRelayUpdate(data: Map<String, String>) {
        val relayUrls = data["relays"]?.split(",") ?: return
        IrohWalletModule.updateRelayServers(relayUrls)
    }
}
```

---

## 4. Cloud Relay Integration

### 4.1 Iroh Relay Architecture

**Citation:** [Iroh Documentation - Relay Servers](https://docs.rs/iroh-net) [^14^]

Iroh uses the DERP (Designated Relay for Encrypted Packets) protocol for NAT traversal and relay:

```
┌─────────────┐                      ┌─────────────┐
│   Mobile    │                      │   Mobile    │
│   Device A  │◄────── DERP ───────►│   Device B  │
│  (behind    │       Relay         │  (behind    │
│   NAT)      │      Server         │   NAT)      │
└─────────────┘                      └─────────────┘
       │                                    │
       └────────── Hole Punching ───────────┘
              (attempt direct connection)
```

### 4.2 Relay Configuration

```rust
// Configure Iroh with relay servers
use iroh::{Endpoint, RelayMode, RelayUrl};

async fn configure_with_relays() -> Result<Endpoint, Box<dyn std::error::Error>> {
    // Public Iroh relay servers
    let relay_urls: Vec<RelayUrl> = vec![
        "https://relay.iroh.network".parse()?,
        "https://euw1-1.derp.iroh.network".parse()?,
        "https://use1-1.derp.iroh.network".parse()?,
    ];

    let endpoint = Endpoint::builder()
        .relay_mode(RelayMode::Custom(relay_urls))
        // Enable discovery for finding peers
        .discovery(true)
        // Bind to available interface
        .bind()
        .await?;

    Ok(endpoint)
}
```

### 4.3 Custom Relay Server

For "Wake-and-Sign" patterns, you may need a custom relay with message queueing:

```rust
// Custom relay with offline message support
use iroh_relay::RelayConfig;

pub struct QueuedRelay {
    // Store messages for offline peers
    message_queue: Arc<RwLock<HashMap<NodeId, Vec<QueuedMessage>>>>,
}

#[derive(Clone, Debug)]
pub struct QueuedMessage {
    pub from: NodeId,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub expires_at: u64,
}

impl QueuedRelay {
    /// Queue a message for an offline peer
    pub async fn queue_message(
        &self,
        to: NodeId,
        message: QueuedMessage,
    ) -> Result<(), RelayError> {
        let mut queue = self.message_queue.write().await;
        
        let messages = queue.entry(to).or_insert_with(Vec::new);
        messages.push(message);
        
        // Limit queue size per peer
        if messages.len() > 100 {
            messages.remove(0);
        }
        
        Ok(())
    }
    
    /// Retrieve queued messages when peer comes online
    pub async fn retrieve_messages(
        &self,
        node_id: NodeId,
    ) -> Vec<QueuedMessage> {
        let mut queue = self.message_queue.write().await;
        queue.remove(&node_id).unwrap_or_default()
    }
    
    /// Clean up expired messages
    pub async fn cleanup_expired(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        let mut queue = self.message_queue.write().await;
        for messages in queue.values_mut() {
            messages.retain(|m| m.expires_at > now);
        }
    }
}
```

### 4.4 Wake-and-Sign Flow

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
       │                        │                        │
```

### 4.5 Push Notification Payloads

**iOS VoIP Push:**
```json
{
    "aps": {
        "alert": null,
        "badge": 0,
        "sound": null,
        "content-available": 1
    },
    "push_type": "voip",
    "sign_request": {
        "request_id": "uuid-v4-string",
        "document_id": "doc-namespace-id",
        "key": "path/to/key",
        "timestamp": 1234567890,
        "expires_at": 1234567950
    }
}
```

**Android FCM High Priority:**
```json
{
    "message": {
        "token": "device-fcm-token",
        "data": {
            "type": "sign_request",
            "request_id": "uuid-v4-string",
            "document_id": "doc-namespace-id",
            "key": "path/to/key"
        },
        "android": {
            "priority": "high",
            "direct_boot_ok": true
        }
    }
}
```

---

## 5. Battery and Network Optimization

### 5.1 Wake-Connect-Sync-Sleep Pattern

**Citation:** [LiteP2P Background Execution Guide](https://litep2p.com/docs/android/background-execution) [^2^]

```
┌─────────────────────────────────────────────────────────────────┐
│                    Wake-Connect-Sync-Sleep Cycle                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Sleep ◄─── Push/Fetch ───► Wake ───► Connect ───► Sync      │
│     ▲                          │         │           │          │
│     │                          │         │           │          │
│     └──────────────────────────┴─────────┴───────────┘          │
│                                                                 │
│   Sleep duration: 15-30 min (configurable)                      │
│   Wake window: 30-60 seconds                                    │
│   Max sync time: 30 seconds (iOS), 10 minutes (Android)         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 Network Optimization Strategies

**Citation:** [Android Network Optimization](https://developer.android.com/develop/connectivity/network-ops/network-access-optimization) [^55^]

```rust
// Batch multiple operations into single connection
pub struct BatchedSync {
    pending_ops: Vec<SyncOp>,
    batch_timer: tokio::time::Interval,
}

impl BatchedSync {
    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                _ = self.batch_timer.tick() => {
                    self.flush_batch().await;
                }
                op = self.op_receiver.recv() => {
                    if let Some(op) = op {
                        self.pending_ops.push(op);
                        if self.pending_ops.len() >= 10 {
                            self.flush_batch().await;
                        }
                    }
                }
            }
        }
    }
    
    async fn flush_batch(&mut self) {
        if self.pending_ops.is_empty() {
            return;
        }
        
        // Open single connection for all operations
        let conn = self.connect().await;
        
        for op in self.pending_ops.drain(..) {
            // Execute batched operation
            if let Err(e) = self.execute_op(&conn, op).await {
                // Queue for retry
            }
        }
        
        // Close connection
        conn.close().await;
    }
}
```

### 5.3 Battery-Aware Connection Management

```rust
// Monitor battery and adapt behavior
pub struct BatteryAwareManager {
    battery_level: u8,
    is_charging: bool,
    is_low_power_mode: bool,
}

impl BatteryAwareManager {
    pub fn should_sync(&self) -> bool {
        // Always sync when charging
        if self.is_charging {
            return true;
        }
        
        // Don't sync below 20% battery
        if self.battery_level < 20 {
            return false;
        }
        
        // Reduce sync frequency in low power mode
        if self.is_low_power_mode {
            // Sync only every 3rd opportunity
            return self.sync_counter % 3 == 0;
        }
        
        true
    }
    
    pub fn sync_interval_seconds(&self) -> u64 {
        match (self.is_charging, self.battery_level, self.is_low_power_mode) {
            (true, _, _) => 60,      // 1 minute when charging
            (false, 50..=100, false) => 300,  // 5 minutes normal
            (false, 20..50, _) => 600,        // 10 minutes low battery
            (false, _, true) => 1800,         // 30 minutes low power mode
            _ => 3600,                        // 1 hour critical
        }
    }
}
```

### 5.4 Connection Pooling

```rust
// Maintain persistent connection pool
pub struct ConnectionPool {
    connections: HashMap<NodeId, PooledConnection>,
    max_connections: usize,
    idle_timeout: Duration,
}

impl ConnectionPool {
    pub async fn get_or_create(&mut self, node_id: NodeId) -> Result<Connection, Error> {
        // Check for existing connection
        if let Some(conn) = self.connections.get(&node_id) {
            if conn.is_alive().await {
                return Ok(conn.clone());
            }
        }
        
        // Evict oldest if at capacity
        if self.connections.len() >= self.max_connections {
            self.evict_oldest().await;
        }
        
        // Create new connection
        let conn = self.establish_connection(node_id).await?;
        self.connections.insert(node_id, PooledConnection::new(conn.clone()));
        
        Ok(conn)
    }
    
    async fn evict_oldest(&mut self) {
        let oldest = self.connections
            .iter()
            .min_by_key(|(_, conn)| conn.last_used)
            .map(|(id, _)| *id);
            
        if let Some(id) = oldest {
            if let Some(conn) = self.connections.remove(&id) {
                let _ = conn.close().await;
            }
        }
    }
}
```

---

## 6. Implementation Checklist

### 6.1 Development Setup

- [ ] Install Rust toolchain with mobile targets
- [ ] Set up `uniffi-bindgen-react-native` in project
- [ ] Create UDL or proc-macro based interface
- [ ] Configure iOS/Android build scripts
- [ ] Set up React Native Turbo Module structure

### 6.2 iOS Implementation

- [ ] Configure Background Modes (VoIP, fetch, remote-notification)
- [ ] Implement PushKit for VoIP pushes
- [ ] Set up BGTaskScheduler for periodic sync
- [ ] Handle silent push notifications
- [ ] Test background execution limits

### 6.3 Android Implementation

- [ ] Configure Foreground Service with dataSync type
- [ ] Implement FCM messaging service
- [ ] Set up WorkManager for scheduled sync
- [ ] Handle Doze mode and App Standby
- [ ] Test on various OEM devices

### 6.4 Relay Integration

- [ ] Configure Iroh with public relay servers
- [ ] Set up custom relay with message queueing (if needed)
- [ ] Implement push notification server
- [ ] Test "Wake-and-Sign" flow end-to-end

### 6.5 Optimization

- [ ] Implement batching for network operations
- [ ] Add battery-aware sync scheduling
- [ ] Set up connection pooling
- [ ] Profile battery usage
- [ ] Optimize for low-power scenarios

---

## 7. References and Citations

### Official Documentation

1. **Mozilla UniFFi**: https://mozilla.github.io/uniffi-rs/ [^1^]
2. **UniFFi React Native Tutorial**: https://jhugman.github.io/uniffi-bindgen-react-native/ [^3^]
3. **UniFFi Async Support**: https://mozilla.github.io/uniffi-rs/0.28/futures.html [^5^]
4. **React Native Turbo Modules**: https://reactnative.dev/docs/turbo-native-modules-introduction [^61^]

### iOS Background Execution

5. **Apple Background Execution Guide**: https://developer.apple.com/forums/thread/685525 [^17^]
6. **Silent Push Notifications**: https://developer.apple.com/documentation/usernotifications/pushing-background-updates-to-your-app [^34^]
7. **WWDC Background Execution**: https://developer.apple.com/videos/play/wwdc2019/707/ [^30^]

### Android Background Execution

8. **Android Background Execution**: https://android-developers.googleblog.com/2018/10/modern-background-execution-in-android.html [^11^]
9. **WorkManager Guide**: https://developer.android.com/develop/background-work/services [^22^]
10. **Network Optimization**: https://developer.android.com/develop/connectivity/network-ops/network-access-optimization [^55^]

### Iroh Documentation

11. **Iroh Net**: https://docs.rs/iroh-net [^16^]
12. **Iroh GitHub**: https://github.com/n0-computer/iroh [^15^]
13. **Iroh FFI**: https://github.com/n0-computer/iroh-ffi [^49^]
14. **Iroh QUIC Multipath**: https://www.iroh.computer/blog/iroh-on-QUIC-multipath [^13^]

### P2P Background Execution

15. **LiteP2P Background Execution**: https://litep2p.com/docs/android/background-execution [^2^]

---

## 8. Conclusion

Running Iroh in a mobile environment is **feasible** with the following considerations:

### Strengths
- ✅ UniFFi for React Native is production-ready
- ✅ Iroh's relay architecture supports mobile NAT traversal
- ✅ Rust's performance and safety are ideal for crypto operations
- ✅ Cloud relay integration enables "Wake-and-Sign" patterns

### Challenges
- ⚠️ Background execution requires push notification infrastructure
- ⚠️ Battery optimization requires careful wake-connect-sync-sleep patterns
- ⚠️ iOS/Android background limitations require platform-specific handling
- ⚠️ Iroh FFI bindings for React Native require custom UniFFi implementation

### Recommendations

1. **Use UniFFi for React Native** for the cleanest integration
2. **Implement VoIP pushes (iOS) and FCM high-priority messages (Android)** for instant wake
3. **Follow wake-connect-sync-sleep patterns** to respect battery constraints
4. **Leverage Iroh's built-in relay servers** for NAT traversal
5. **Implement connection pooling and batching** for network efficiency
6. **Test extensively on real devices** under various battery and network conditions

---

*Report generated for Local-First Identity Wallet project*
*Analysis covers iOS, Android, and React Native integration patterns*
