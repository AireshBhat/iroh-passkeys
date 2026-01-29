# LLM Agent Coding Rules: Iroh Personal Node

## Document Information
- **Version**: 1.0.0
- **Status**: Active
- **Last Updated**: 2026-01-29
- **Applies To**: All code generation for this project

---

## 1. Core Principles

### 1.1 Security First
```
PRIORITY 1: NEVER compromise security for convenience
- All cryptographic keys must be zeroized after use
- Sensitive data must never be logged or displayed
- Use constant-time operations for comparisons involving secrets
- Prefer explicit error handling over unwrap() in cryptographic code
```

### 1.2 Minimal Attack Surface
```
- Include only necessary dependencies
- Disable unused crate features (especially for WASM)
- Avoid unsafe code unless absolutely necessary
- Keep WASM bundle size minimal
```

### 1.3 Deterministic Behavior
```
- Same input must always produce same output
- Document all sources of randomness
- Use explicit seeds for any non-cryptographic RNG
```

---

## 2. Rust Code Rules

### 2.1 Error Handling

**ALWAYS use proper error handling - NEVER use unwrap() in production code.**

```rust
// ❌ BAD
let result = some_operation().unwrap();

// ✅ GOOD
let result = some_operation()
    .map_err(|e| WalletError::OperationFailed(e.to_string()))?;

// ✅ GOOD - with context
let result = some_operation()
    .context("failed to perform operation")?;
```

**Use thiserror for error types:**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
    
    #[error("Node initialization failed: {0}")]
    NodeInitFailed(#[from] iroh::EndpointError),
    
    #[error("Invalid input: expected {expected}, got {actual}")]
    InvalidInput { expected: String, actual: String },
}
```

### 2.2 WASM Bindings

**Use wasm-bindgen properly for JavaScript interop:**

```rust
use wasm_bindgen::prelude::*;

// ❌ BAD - not exposed to JS
pub fn internal_function() {}

// ✅ GOOD - properly exposed
#[wasm_bindgen]
pub fn public_function() {}

// ✅ GOOD - async support
#[wasm_bindgen]
pub async fn async_function() -> Result<JsValue, JsValue> {
    // Implementation
}

// ✅ GOOD - custom JS name
#[wasm_bindgen(js_name = "deriveTransportKey")]
pub fn derive_transport_key(...) {}
```

**Handle JavaScript types carefully:**

```rust
// ❌ BAD - raw pointer
#[wasm_bindgen]
pub fn process_bytes(ptr: *const u8, len: usize) {}

// ✅ GOOD - use js_sys types
#[wasm_bindgen]
pub fn process_bytes(data: &[u8]) -> Box<[u8]> {}

// ✅ GOOD - for Uint8Array
use js_sys::Uint8Array;

#[wasm_bindgen]
pub fn process_array(arr: &Uint8Array) -> Uint8Array {
    let vec = arr.to_vec();
    // Process...
    Uint8Array::from(&result[..])
}
```

### 2.3 Memory Safety & Zeroization

**ALWAYS zeroize sensitive data:**

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

// ✅ GOOD - automatic zeroization
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DerivedKeys {
    #[zeroize(skip)]  // Public data doesn't need zeroization
    pub node_id: String,
    
    // Sensitive fields are automatically zeroized on drop
    pub transport_key: [u8; 32],
    pub identity_key: [u8; 32],
}

// ✅ GOOD - explicit zeroization
impl Drop for SecureContainer {
    fn drop(&mut self) {
        self.key.zeroize();
        self.secret.zeroize();
    }
}

// ✅ GOOD - zeroize before shutdown
pub async fn shutdown(&mut self) {
    if let Some(endpoint) = self.endpoint.take() {
        endpoint.close().await;
    }
    self.keys.zeroize();
}
```

### 2.4 Iroh Integration

**Configure Iroh for WASM/browser environment:**

```rust
// ✅ GOOD - WASM-compatible configuration
use iroh::RelayMode;

let endpoint = iroh::Endpoint::builder()
    .secret_key(secret_key)
    .relay_mode(RelayMode::Enabled)  // Required for WASM
    .discovery_n0()                   // Enable discovery
    .bind()
    .await?;
```

**Handle Iroh types properly:**

```rust
// ✅ GOOD - convert to string for JS
#[wasm_bindgen]
impl IrohNode {
    #[wasm_bindgen(getter)]
    pub fn node_id(&self) -> String {
        self.endpoint.node_id().to_string()
    }
}

// ✅ GOOD - handle NodeAddr parsing
pub async fn connect(&self, addr: String) -> Result<Connection, WalletError> {
    let node_addr: iroh::NodeAddr = addr.parse()
        .map_err(|e| WalletError::InvalidAddress(e.to_string()))?;
    
    self.endpoint.connect(node_addr, b"iroh-wallet/1")
        .await
        .map_err(|e| WalletError::ConnectionFailed(e.to_string()))
}
```

### 2.5 Cryptographic Operations

**Use standard, well-reviewed crates:**

```rust
// ✅ GOOD - HKDF per RFC 5869
use hkdf::Hkdf;
use sha2::Sha256;

pub fn derive_key(ikm: &[u8], salt: &[u8], info: &[u8]) -> Result<[u8; 32], String> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut okm = [0u8; 32];
    hkdf.expand(info, &mut okm)
        .map_err(|e| format!("HKDF expansion failed: {}", e))?;
    Ok(okm)
}

// ✅ GOOD - constant-time comparison
use subtle::ConstantTimeEq;

pub fn verify_mac(mac1: &[u8], mac2: &[u8]) -> bool {
    mac1.ct_eq(mac2).into()
}
```

### 2.6 Code Organization

**Follow module structure:**

```
src/
├── lib.rs           # Public API, WASM exports
├── error.rs         # Error types
├── keys.rs          # Key derivation
├── node.rs          # Iroh node management
├── crypto.rs        # Cryptographic utilities
└── utils.rs         # Helper functions
```

**Use explicit imports:**

```rust
// ❌ BAD - wildcard imports
use crate::*;

// ✅ GOOD - explicit imports
use crate::error::WalletError;
use crate::keys::{derive_transport_key, DerivedKeys};
use crate::node::IrohNode;
```

---

## 3. JavaScript/TypeScript Rules

### 3.1 WASM Module Loading

**Load WASM correctly with proper error handling:**

```javascript
// ✅ GOOD - proper initialization
async function initWasm() {
    try {
        const wasm = await import('./pkg/iroh_wallet_wasm.js');
        await wasm.default();  // Initialize the module
        return wasm;
    } catch (err) {
        console.error('Failed to load WASM module:', err);
        throw new Error('Your browser may not support WebAssembly');
    }
}

// ✅ GOOD - singleton pattern
let wasmModule = null;

export async function getWasmModule() {
    if (!wasmModule) {
        wasmModule = await initWasm();
    }
    return wasmModule;
}
```

### 3.2 State Management

**Keep sensitive data in WASM memory only:**

```javascript
// ❌ BAD - storing keys in JavaScript
const masterSeed = new Uint8Array([...]);  // Accessible to XSS
localStorage.setItem('key', masterSeed);   // NEVER do this

// ✅ GOOD - pass to WASM immediately, don't retain
async function authenticate() {
    const masterSeed = await getPrfOutput();
    const transportKey = wasm.derive_transport_key(masterSeed);
    // Clear from JS memory
    masterSeed.fill(0);
    
    // Store only in WASM
    return await new wasm.IrohNode(transportKey);
}
```

### 3.3 WebAuthn Implementation

**Follow WebAuthn best practices:**

```javascript
// ✅ GOOD - proper PRF usage
async function authenticateWithPrf() {
    const salt = await getStoredSalt();  // 32 bytes
    
    const options = {
        publicKey: {
            challenge: crypto.getRandomValues(new Uint8Array(32)),
            rpId: location.hostname,
            allowCredentials: [],  // Discoverable credentials
            userVerification: 'required',
            extensions: {
                prf: {
                    eval: { first: salt }
                }
            }
        }
    };
    
    try {
        const credential = await navigator.credentials.get(options);
        const results = credential.getClientExtensionResults();
        
        if (!results.prf?.results?.first) {
            throw new Error('PRF results not available');
        }
        
        return new Uint8Array(results.prf.results.first);
    } catch (err) {
        if (err.name === 'NotAllowedError') {
            throw new Error('Authentication cancelled by user');
        }
        throw err;
    }
}
```

### 3.4 Error Handling

**Convert WASM errors to user-friendly messages:**

```javascript
// ✅ GOOD - error translation
function handleWasmError(error) {
    const errorMap = {
        'NodeInitFailed': 'Failed to start the node. Please check your connection.',
        'InvalidKey': 'The provided key is invalid. Please try authenticating again.',
        'ConnectionFailed': 'Could not connect to the peer. They may be offline.',
    };
    
    const code = error.code || 'UnknownError';
    return errorMap[code] || `An error occurred: ${error.message}`;
}

// ✅ GOOD - UI error display
async function initNode() {
    try {
        await node.initialize();
    } catch (err) {
        const message = handleWasmError(err);
        showError(message);
        logError(err);  // For debugging
    }
}
```

---

## 4. HTML/CSS Rules

### 4.1 Mobile-First Design

**Design for mobile screens first:**

```css
/* ✅ GOOD - mobile-first media queries */
.container {
    padding: 16px;
    max-width: 100%;
}

/* Tablet and up */
@media (min-width: 768px) {
    .container {
        max-width: 600px;
        margin: 0 auto;
    }
}
```

**Ensure touch-friendly sizes:**

```css
/* ✅ GOOD - minimum touch target size */
button {
    min-height: 44px;
    min-width: 44px;
    padding: 12px 24px;
}

/* ✅ GOOD - adequate spacing */
.form-field {
    margin-bottom: 16px;
}
```

### 4.2 PWA Requirements

**Include required PWA elements:**

```html
<!-- ✅ GOOD - viewport meta -->
<meta name="viewport" content="width=device-width, initial-scale=1.0">

<!-- ✅ GOOD - theme color -->
<meta name="theme-color" content="#0a0a0a">

<!-- ✅ GOOD - manifest -->
<link rel="manifest" href="/manifest.json">

<!-- ✅ GOOD - icons -->
<link rel="icon" href="/favicon.ico">
<link rel="apple-touch-icon" href="/icon-192.png">
```

### 4.3 Loading Performance

**Optimize asset loading:**

```html
<!-- ✅ GOOD - async non-critical scripts -->
<script type="module" src="/app.js" async></script>

<!-- ✅ GOOD - preload critical resources -->
<link rel="preload" href="/pkg/iroh_wallet_wasm_bg.wasm" as="fetch" crossorigin>

<!-- ✅ GOOD - inline critical CSS -->
<style>
    /* Critical styles here */
</style>
```

---

## 5. Security Rules

### 5.1 Content Security Policy

**Implement strict CSP:**

```html
<!-- ✅ GOOD - strict CSP for WASM -->
<meta http-equiv="Content-Security-Policy" content="
    default-src 'self';
    script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval';
    style-src 'self' 'unsafe-inline';
    connect-src 'self' wss://relay.iroh.network https://iroh.network;
    img-src 'self' data:;
">
```

### 5.2 Secure Context Requirements

**Verify secure context for WebAuthn:**

```javascript
// ✅ GOOD - check secure context
if (!window.isSecureContext) {
    showError('This application requires a secure context (HTTPS or localhost)');
}

// ✅ GOOD - check WebAuthn availability
if (!window.PublicKeyCredential) {
    showError('WebAuthn is not supported in this browser');
}
```

### 5.3 Input Validation

**Validate all inputs before processing:**

```rust
// ✅ GOOD - input validation
pub fn derive_transport_key(master_seed: &[u8]) -> Result<Box<[u8]>, JsValue> {
    if master_seed.len() != 32 {
        return Err(JsValue::from_str(
            &format!("Invalid seed length: expected 32, got {}", master_seed.len())
        ));
    }
    // ... derivation
}
```

---

## 6. Testing Rules

### 6.1 Unit Tests

**Test edge cases and error conditions:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ✅ GOOD - test success case
    #[test]
    fn test_derive_key_success() {
        let seed = [0x42u8; 32];
        let result = derive_transport_key(&seed);
        assert!(result.is_ok());
    }

    // ✅ GOOD - test error case
    #[test]
    fn test_derive_key_wrong_length() {
        let seed = [0x42u8; 16];  // Too short
        let result = derive_transport_key(&seed);
        assert!(result.is_err());
    }

    // ✅ GOOD - test determinism
    #[test]
    fn test_derive_key_deterministic() {
        let seed = [0x42u8; 32];
        let key1 = derive_transport_key(&seed).unwrap();
        let key2 = derive_transport_key(&seed).unwrap();
        assert_eq!(key1.as_ref(), key2.as_ref());
    }
}
```

### 6.2 WASM Tests

**Use wasm-bindgen-test for browser tests:**

```rust
#[cfg(test)]
mod wasm_tests {
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);

    // ✅ GOOD - test in browser environment
    #[wasm_bindgen_test]
    async fn test_node_init() {
        let keys = [0u8; 32];
        let node = IrohNode::new(Box::new(keys)).await;
        assert!(node.is_ok());
    }
}
```

---

## 7. Documentation Rules

### 7.1 Rust Documentation

**Document all public APIs:**

```rust
/// Derives a transport key from a 32-byte master seed using HKDF-SHA256.
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
/// ```
/// let seed = [0x42u8; 32];
/// let key = derive_transport_key(&seed).expect("valid derivation");
/// assert_eq!(key.len(), 32);
/// ```
#[wasm_bindgen]
pub fn derive_transport_key(master_seed: &[u8]) -> Result<Box<[u8]>, JsValue> {
    // Implementation
}
```

### 7.2 JavaScript Documentation

**Use JSDoc for JavaScript functions:**

```javascript
/**
 * Authenticates the user using WebAuthn PRF and derives the master seed.
 *
 * @async
 * @param {Uint8Array} salt - A 32-byte salt for PRF evaluation
 * @returns {Promise<Uint8Array>} The 32-byte master seed
 * @throws {Error} If authentication fails or PRF is not supported
 *
 * @example
 * const salt = crypto.getRandomValues(new Uint8Array(32));
 * const seed = await authenticateWithPrf(salt);
 */
async function authenticateWithPrf(salt) {
    // Implementation
}
```

---

## 8. Git Commit Rules

### 8.1 Commit Message Format

**Use conventional commits:**

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting changes (no code change)
- `refactor`: Code refactoring
- `test`: Test additions/changes
- `chore`: Build process or auxiliary tool changes

**Examples:**

```
feat(keys): implement HKDF key derivation

- Add derive_transport_key function
- Use HKDF-SHA256 per RFC 5869
- Zeroize intermediate values

Closes #123
```

```
fix(node): handle connection timeout gracefully

Previously, connection timeouts would panic. Now they return
WalletError::Timeout with a descriptive message.

Fixes #456
```

---

## 9. Forbidden Patterns

### 9.1 Security Anti-patterns

```rust
// ❌ NEVER - unwrap in crypto code
let key = derive_key(&seed).unwrap();

// ❌ NEVER - log sensitive data
eprintln!("Derived key: {:?}", key);

// ❌ NEVER - store keys in String
let key_string = String::from_utf8(key.to_vec());

// ❌ NEVER - use unsafe without comment
unsafe { /* ... */ }
```

### 9.2 WASM Anti-patterns

```rust
// ❌ NEVER - expose raw pointers
#[wasm_bindgen]
pub fn get_key_ptr(&self) -> *const u8 { }

// ❌ NEVER - use std::time in WASM
let now = std::time::Instant::now();  // May panic

// ❌ NEVER - use std::fs in WASM
let file = std::fs::read("config.txt");  // Won't work
```

### 9.3 JavaScript Anti-patterns

```javascript
// ❌ NEVER - store keys in localStorage
localStorage.setItem('masterSeed', seed);

// ❌ NEVER - use eval()
eval(userInput);

// ❌ NEVER - disable security warnings
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';
```

---

## 10. Checklist for Code Generation

Before submitting generated code, verify:

### Rust Code
- [ ] All `unwrap()` calls are justified and documented
- [ ] All errors are properly propagated
- [ ] Sensitive data uses `Zeroize`/`ZeroizeOnDrop`
- [ ] WASM bindings use correct types
- [ ] Async functions return `Result` with proper error types
- [ ] No `unsafe` code without safety comments
- [ ] All public functions have documentation
- [ ] Tests cover success and error cases

### JavaScript Code
- [ ] No sensitive data stored in global scope
- [ ] WASM errors are handled and translated
- [ ] No `eval()` or `new Function()` usage
- [ ] CSP-compatible (no inline event handlers)
- [ ] Feature detection before API usage
- [ ] Graceful degradation for unsupported browsers

### HTML/CSS
- [ ] Mobile-responsive design
- [ ] Proper viewport meta tag
- [ ] Touch targets minimum 44px
- [ ] Works without JavaScript (graceful degradation)
- [ ] PWA manifest included
- [ ] CSP meta tag present

---

## 11. Reference Resources

When generating code, consult these resources:

1. **Iroh Documentation**: https://docs.rs/iroh/
2. **wasm-bindgen Guide**: https://rustwasm.github.io/wasm-bindgen/
3. **WebAuthn Spec**: https://w3c.github.io/webauthn/
4. **HKDF RFC 5869**: https://tools.ietf.org/html/rfc5869
5. **Rust WASM Book**: https://rustwasm.github.io/book/

---

**Remember: When in doubt, prioritize security over convenience. Ask for clarification if requirements are unclear.**
