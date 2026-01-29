# Project Tasks: Iroh Personal Node (Mobile Browser MVP)

## Document Information
- **Version**: 1.0.0
- **Status**: Draft
- **Last Updated**: 2026-01-29
- **Format**: Epic > Story > Task > Subtask

---

## Epic Overview

| ID | Epic | Priority | Status | Est. Effort |
|----|------|----------|--------|-------------|
| E1 | Foundation & Tooling | P0 | 🔵 Not Started | 2 days |
| E2 | Rust WASM Core | P0 | 🔵 Not Started | 5 days |
| E3 | Web UI & Authentication | P0 | 🔵 Not Started | 4 days |
| E4 | Testing & Quality | P1 | 🔵 Not Started | 3 days |
| E5 | Deployment & DevOps | P1 | 🔵 Not Started | 2 days |
| E6 | Documentation | P1 | 🔵 Not Started | 2 days |
| E7 | Future Enhancements | P2 | ⚪ Backlog | 8 days |

**Legend:**
- 🔵 Not Started
- 🟡 In Progress
- 🟢 Completed
- 🔴 Blocked
- ⚪ Backlog

---

## Epic E1: Foundation & Tooling

**Objective**: Set up the project structure, build tools, and development environment.

**Acceptance Criteria**:
- [ ] Repository structure follows Rust/WASM conventions
- [ ] Build scripts automate WASM generation
- [ ] Development server hot-reloads on changes
- [ ] CI pipeline validates builds

---

### Story E1-S1: Project Initialization
**Priority**: P0 | **Est.**: 4h

- [ ] **T1.1** Create project directory structure
  - [ ] Create `src/` for Rust source code
  - [ ] Create `www/` for web assets
  - [ ] Create `scripts/` for build utilities
  - [ ] Create `tests/` for test files
  - [ ] Create `docs/` for additional documentation

- [ ] **T1.2** Initialize Rust project
  - [ ] Run `cargo init --lib`
  - [ ] Configure `Cargo.toml` with WASM target settings
  - [ ] Add `.cargo/config.toml` for build configuration
  - [ ] Create `rust-toolchain.toml` specifying Rust version

- [ ] **T1.3** Set up version control
  - [ ] Initialize git repository
  - [ ] Create `.gitignore` for Rust/WASM projects
  - [ ] Create initial commit with project structure

---

### Story E1-S2: Build System Setup
**Priority**: P0 | **Est.**: 6h

- [ ] **T1.4** Install and configure wasm-pack
  - [ ] Add wasm-pack installation script
  - [ ] Configure `wasm-pack.toml` if needed
  - [ ] Test `wasm-pack build` succeeds

- [ ] **T1.5** Create build automation scripts
  - [ ] Create `build.sh` for Unix systems
  - [ ] Create `build.ps1` for Windows
  - [ ] Add watch mode for development (`watch.sh`)
  - [ ] Create `clean.sh` to reset build artifacts

- [ ] **T1.6** Configure target architectures
  - [ ] Add `wasm32-unknown-unknown` target
  - [ ] Test release vs debug builds
  - [ ] Verify output in `pkg/` directory

---

### Story E1-S3: Development Environment
**Priority**: P0 | **Est.**: 4h

- [ ] **T1.7** Set up local development server
  - [ ] Configure Node serve
  - [ ] Enable CORS headers for WASM MIME types
  - [ ] Add hot-reload on file changes
  - [ ] Create `start-dev.sh` script

- [ ] **T1.8** Configure IDE/editor settings
  - [ ] Create `.vscode/settings.json` (if using VS Code)
  - [ ] Add recommended extensions list
  - [ ] Configure rust-analyzer for WASM targets
  - [ ] Add debugging configuration

- [ ] **T1.9** Set up linting and formatting
  - [ ] Configure `rustfmt.toml`
  - [ ] Add `clippy` configuration
  - [ ] Create pre-commit hooks (optional)

---

## Epic E2: Rust WASM Core

**Objective**: Implement the Rust library with WASM bindings for Iroh node operation.

**Acceptance Criteria**:
- [ ] Library compiles to WASM without errors
- [ ] All functions exposed via wasm-bindgen
- [ ] Zeroization of sensitive data implemented
- [ ] Error handling propagates to JavaScript

---

### Story E2-S1: Dependencies & Configuration
**Priority**: P0 | **Est.**: 4h

- [ ] **T2.1** Configure Cargo.toml dependencies
  - [ ] Add `iroh = { version = "0.33", default-features = false }`
  - [ ] Add `wasm-bindgen = "0.2"` with features
  - [ ] Add `wasm-bindgen-futures = "0.4"`
  - [ ] Add `js-sys` and `web-sys` with required features
  - [ ] Add crypto crates: `hkdf`, `sha2`, `ed25519-dalek`
  - [ ] Add `zeroize` with derive feature
  - [ ] Add `console_error_panic_hook` for debugging
  - [ ] Add `wee_alloc` for smaller bundle (optional)

- [ ] **T2.2** Create build configuration
  - [ ] Configure `build.rs` if needed
  - [ ] Set up `cfg_aliases` for WASM browser target
  - [ ] Add conditional compilation for wasm32
  - [ ] Verify dependency tree compiles

- [ ] **T2.3** Initialize library structure
  - [ ] Create `src/lib.rs` with module declarations
  - [ ] Add `wasm_bindgen(start)` initialization
  - [ ] Set up panic hook for better errors
  - [ ] Add logging integration with `web-sys`

---

### Story E2-S2: Key Derivation Module
**Priority**: P0 | **Est.**: 8h

- [ ] **T2.4** Implement HKDF constants
  - [ ] Define `APP_SALT` constant
  - [ ] Define `INFO_TRANSPORT_KEY` constant
  - [ ] Define `INFO_IDENTITY_KEY` constant (future use)
  - [ ] Define `INFO_STORAGE_KEY` constant (future use)
  - [ ] Document constant purposes

- [ ] **T2.5** Create key derivation function
  - [ ] Implement `derive_transport_key()` function
  - [ ] Add input validation (32-byte check)
  - [ ] Implement HKDF-SHA256 extract phase
  - [ ] Implement HKDF-SHA256 expand phase
  - [ ] Return `Box<[u8]>` for WASM compatibility
  - [ ] Add comprehensive error handling

- [ ] **T2.6** Create key container types
  - [ ] Define `DerivedKeys` struct with `ZeroizeOnDrop`
  - [ ] Implement conversion methods
  - [ ] Add secure memory clearing on drop
  - [ ] Create unit tests for derivation

- [ ] **T2.7** Add key derivation tests
  - [ ] Test deterministic output (same input = same output)
  - [ ] Test different info strings produce different keys
  - [ ] Test error handling for invalid inputs
  - [ ] Verify zeroization works correctly

---

### Story E2-S3: Iroh Node Module
**Priority**: P0 | **Est.**: 12h

- [ ] **T2.8** Define error types
  - [ ] Create `WalletError` enum with wasm_bindgen
  - [ ] Implement `Display` for error messages
  - [ ] Map Iroh errors to WalletError variants
  - [ ] Add JavaScript-compatible error conversion

- [ ] **T2.9** Implement IrohNode struct
  - [ ] Define struct with endpoint and metadata fields
  - [ ] Derive `wasm_bindgen` for the struct
  - [ ] Handle Option<Endpoint> for lifecycle management
  - [ ] Add internal state tracking

- [ ] **T2.10** Implement node constructor
  - [ ] Create `new()` async constructor
  - [ ] Parse 32-byte transport key
  - [ ] Build Iroh SecretKey from bytes
  - [ ] Configure Endpoint builder with relay mode
  - [ ] Set up discovery (N0)
  - [ ] Call bind() and handle errors
  - [ ] Store endpoint in struct
  - [ ] Extract and store node_id

- [ ] **T2.11** Implement node info methods
  - [ ] Create `node_id()` getter
  - [ ] Implement `is_connected()` check
  - [ ] Add `relay_address()` method (if available)
  - [ ] Return connection statistics

- [ ] **T2.12** Implement shutdown
  - [ ] Create `shutdown()` async method
  - [ ] Close endpoint gracefully
  - [ ] Zeroize sensitive data
  - [ ] Set state to disconnected
  - [ ] Handle already-shutdown case

- [ ] **T2.13** Add connection methods (basic)
  - [ ] Implement `connect(node_addr: String)` stub
  - [ ] Parse NodeAddr from string
  - [ ] Return connection handle or error
  - [ ] Add connection timeout handling

---

### Story E2-S4: WASM Bindings & FFI
**Priority**: P0 | **Est.**: 6h

- [ ] **T2.14** Expose functions to JavaScript
  - [ ] Add `#[wasm_bindgen]` to all public functions
  - [ ] Handle async with `#[wasm_bindgen(js_name = ...)]`
  - [ ] Convert Rust types to JS-compatible types
  - [ ] Test function calls from JavaScript

- [ ] **T2.15** Handle JavaScript types
  - [ ] Convert `Uint8Array` to `&[u8]` and `Box<[u8]>`
  - [ ] Handle `Promise`/`Future` conversions
  - [ ] Manage string encoding (UTF-8)
  - [ ] Test type round-trips

- [ ] **T2.16** Add TypeScript definitions (optional)
  - [ ] Generate `.d.ts` files if using wasm-pack
  - [ ] Verify type definitions match implementation
  - [ ] Document any manual type definitions needed

---

### Story E2-S5: Security & Memory Safety
**Priority**: P0 | **Est.**: 4h

- [ ] **T2.17** Implement zeroization
  - [ ] Apply `ZeroizeOnDrop` to all key-holding structs
  - [ ] Ensure keys cleared on drop
  - [ ] Add explicit zeroize calls before shutdown
  - [ ] Test memory doesn't leak keys

- [ ] **T2.18** Add constant-time operations
  - [ ] Use constant-time comparison where needed
  - [ ] Avoid timing side-channels
  - [ ] Document security assumptions

- [ ] **T2.19** Secure error messages
  - [ ] Don't leak key material in errors
  - [ ] Sanitize debug output
  - [ ] Review all error paths for info leakage

---

## Epic E3: Web UI & Authentication

**Objective**: Create the web interface with WebAuthn PRF authentication and node controls.

**Acceptance Criteria**:
- [ ] UI is responsive for mobile devices
- [ ] WebAuthn PRF works on supported browsers
- [ ] Fallback works when PRF unavailable
- [ ] Node status is clearly displayed
- [ ] All errors are shown to user

---

### Story E3-S1: Static Assets & Structure
**Priority**: P0 | **Est.**: 4h

- [ ] **T3.1** Create HTML structure
  - [ ] Create `www/index.html` with semantic markup
  - [ ] Add viewport meta for mobile
  -[ ] Include WASM script loader
  - [ ] Add placeholder for dynamic content

- [ ] **T3.2** Add CSS styling
  - [ ] Create mobile-first responsive styles
  - [ ] Add dark mode theme
  - [ ] Style status indicators (online/offline)
  - [ ] Add button states (hover, active, disabled)
  - [ ] Ensure touch targets are 44px minimum

- [ ] **T3.3** Add PWA support
  - [ ] Create `manifest.json`
  - [ ] Add icon files (192x192, 512x512)
  - [ ] Add theme-color meta tag
  - [ ] Test add-to-home-screen functionality

- [ ] **T3.4** Add favicon and assets
  - [ ] Create favicon.ico
  - [ ] Add apple-touch-icon
  - [ ] Verify all assets load correctly

---

### Story E3-S2: WASM Module Loading
**Priority**: P0 | **Est.**: 3h

- [ ] **T3.5** Implement module initialization
  - [ ] Create ES module script in HTML
  - [ ] Import WASM init function
  - [ ] Handle initialization promise
  - [ ] Show loading state to user
  - [ ] Handle initialization errors

- [ ] **T3.6** Create JavaScript state management
  - [ ] Define global state object
  - [ ] Track initialization status
  - [ ] Store derived keys reference
  - [ ] Track node instance
  - [ ] Add state change listeners

- [ ] **T3.7** Add error handling for WASM
  - [ ] Catch and display init errors
  - [ ] Handle browser compatibility issues
  - [ ] Provide fallback messaging
  - [ ] Log errors for debugging

---

### Story E3-S3: WebAuthn PRF Authentication
**Priority**: P0 | **Est.**: 8h

- [ ] **T3.8** Implement PRF capability detection
  - [ ] Check for `PublicKeyCredential` API
  - [ ] Detect `getClientCapabilities` method
  - [ ] Test for PRF in extensions list
  - [ ] Store capability flags

- [ ] **T3.9** Implement authentication flow
  - [ ] Create `authenticate()` function
  - [ ] Generate random challenge
  - [ ] Set up credential request options
  - [ ] Add PRF eval with salt
  - [ ] Call `navigator.credentials.get()`
  - [ ] Handle user cancellation

- [ ] **T3.10** Extract PRF results
  - [ ] Parse `getClientExtensionResults()`
  - [ ] Extract `prf.results.first`
  - [ ] Convert ArrayBuffer to Uint8Array
  - [ ] Validate 32-byte length
  - [ ] Store master seed securely

- [ ] **T3.11** Add fallback for unsupported browsers
  - [ ] Detect PRF unavailability
  - [ ] Show fallback message
  - [ ] Implement demo mode with random keys
  - [ ] Warn user about demo limitations
  - [ ] Document browser requirements

- [ ] **T3.12** Handle authentication errors
  - [ ] Handle `NotAllowedError` (user cancelled)
  - [ ] Handle `SecurityError` (invalid context)
  - [ ] Handle `NotSupportedError` (no authenticator)
  - [ ] Show user-friendly error messages
  - [ ] Provide retry mechanism

---

### Story E3-S4: Node Control Interface
**Priority**: P0 | **Est.**: 6h

- [ ] **T3.13** Create node initialization UI
  - [ ] Add "Initialize Node" button
  - [ ] Disable until authenticated
  - [ ] Show loading state during init
  - [ ] Call WASM `derive_transport_key()`
  - [ ] Call WASM `IrohNode.new()`
  - [ ] Handle initialization errors

- [ ] **T3.14** Implement status display
  - [ ] Create status indicator component
  - [ ] Show online/offline state
  - [ ] Display node ID
  - [ ] Show connection type (relay)
  - [ ] Add copy-to-clipboard for node ID

- [ ] **T3.15** Create shutdown functionality
  - [ ] Add "Shutdown Node" button
  - [ ] Call WASM `shutdown()` method
  - [ ] Clear node instance
  - [ ] Reset UI to initial state
  - [ ] Handle shutdown errors

- [ ] **T3.16** Add activity logging
  - [ ] Create log display component
  - [ ] Implement `log(message, type)` function
  - [ ] Auto-scroll to latest entries
  - [ ] Color-code by type (info, success, error)
  - [ ] Add timestamps
  - [ ] Limit log history (keep last 100)

---

### Story E3-S5: Responsive & UX Polish
**Priority**: P1 | **Est.**: 4h

- [ ] **T3.17** Test mobile responsiveness
  - [ ] Test on iOS Safari
  - [ ] Test on Android Chrome
  - [ ] Verify touch interactions work
  - [ ] Check font sizes are readable
  - [ ] Ensure no horizontal scroll

- [ ] **T3.18** Add loading states
  - [ ] Show spinner during WASM init
  - [ ] Show spinner during node init
  - [ ] Disable buttons during operations
  - [ ] Add progress indicators where appropriate

- [ ] **T3.19** Add keyboard accessibility
  - [ ] Ensure all buttons are focusable
  - [ ] Add focus indicators
  - [ ] Test Tab navigation
  - [ ] Add ARIA labels where needed

---

## Epic E4: Testing & Quality

**Objective**: Ensure the system works correctly across browsers and handles edge cases.

**Acceptance Criteria**:
- [ ] Unit tests cover core logic
- [ ] Integration tests verify WASM bindings
- [ ] Manual testing on target browsers
- [ ] Security review completed

---

### Story E4-S1: Rust Unit Tests
**Priority**: P1 | **Est.**: 6h

- [ ] **T4.1** Test key derivation
  - [ ] Test deterministic output
  - [ ] Test different inputs produce different outputs
  - [ ] Test error cases (wrong input size)
  - [ ] Test edge cases (all zeros, all 0xFF)

- [ ] **T4.2** Test IrohNode creation
  - [ ] Test successful initialization
  - [ ] Test with invalid key
  - [ ] Test double initialization prevention
  - [ ] Test shutdown behavior

- [ ] **T4.3** Test error handling
  - [ ] Test error conversion
  - [ ] Test error message formatting
  - [ ] Test error propagation to JS boundary

- [ ] **T4.4** Set up test runner
  - [ ] Configure `cargo test` for wasm target
  - [ ] Add wasm-bindgen-test for browser tests
  - [ ] Create test utilities/helpers

---

### Story E4-S2: WASM Integration Tests
**Priority**: P1 | **Est.**: 4h

- [ ] **T4.5** Test WASM module loading
  - [ ] Verify module initializes in browser
  - [ ] Test error handling on init failure
  - [ ] Verify all exports are available

- [ ] **T4.6** Test JavaScript bindings
  - [ ] Test function calls work
  - [ ] Test async/promise behavior
  - [ ] Test error propagation to JS
  - [ ] Test type conversions

- [ ] **T4.7** Test WebAuthn integration
  - [ ] Mock PRF responses for testing
  - [ ] Test auth flow end-to-end
  - [ ] Test fallback behavior

---

### Story E4-S3: Browser Compatibility Testing
**Priority**: P1 | **Est.**: 6h

- [ ] **T4.8** Test on iOS
  - [ ] Test on iOS 18+ Safari (PRF supported)
  - [ ] Test on iOS Chrome
  - [ ] Test add-to-home-screen
  - [ ] Document any issues

- [ ] **T4.9** Test on Android
  - [ ] Test on Android Chrome (PRF supported)
  - [ ] Test on Android Firefox
  - [ ] Test on Samsung Internet
  - [ ] Document any issues

- [ ] **T4.10** Test on Desktop (for development)
  - [ ] Test on Chrome
  - [ ] Test on Firefox
  - [ ] Test on Safari
  - [ ] Verify responsive design

- [ ] **T4.11** Create compatibility matrix
  - [ ] Document supported browsers
  - [ ] Note PRF support requirements
  - [ ] Document known limitations

---

### Story E4-S4: Security Testing
**Priority**: P1 | **Est.**: 4h

- [ ] **T4.12** Test key zeroization
  - [ ] Verify keys cleared after use
  - [ ] Test shutdown cleanup
  - [ ] Verify no key leakage in memory

- [ ] **T4.13** Test error message safety
  - [ ] Ensure no key material in errors
  - [ ] Review all error paths
  - [ ] Test with malformed inputs

- [ ] **T4.14** Review dependencies
  - [ ] Audit Cargo dependencies
  - [ ] Check for known vulnerabilities
  - [ ] Verify license compatibility

---

## Epic E5: Deployment & DevOps

**Objective**: Deploy the application to a publicly accessible hosting platform.

**Acceptance Criteria**:
- [ ] Application deployed and accessible via HTTPS
- [ ] Build process is automated
- [ ] Documentation for deployment process

---

### Story E5-S1: Build Optimization
**Priority**: P1 | **Est.**: 3h

- [ ] **T5.1** Optimize WASM bundle size
  - [ ] Enable LTO in release profile
  - [ ] Use `wasm-opt` for optimization
  - [ ] Strip debug symbols
  - [ ] Verify size is acceptable (< 5MB)

- [ ] **T5.2** Optimize web assets
  - [ ] Minify CSS
  - [ ] Minify JavaScript
  - [ ] Enable gzip/brotli compression
  - [ ] Optimize images

- [ ] **T5.3** Verify production build
  - [ ] Test production build locally
  - [ ] Verify all features work
  - [ ] Check console for errors

---

### Story E5-S2: Hosting Setup
**Priority**: P1 | **Est.**: 3h

- [ ] **T5.4** Configure static hosting
  - [ ] Choose hosting provider (GitHub Pages/Netlify/Vercel)
  - [ ] Set up repository for deployment
  - [ ] Configure build command
  - [ ] Set output directory

- [ ] **T5.5** Configure HTTPS
  - [ ] Verify HTTPS is enabled
  - [ ] Test WebAuthn works (requires secure context)
  - [ ] Configure HSTS headers

- [ ] **T5.6** Set up custom domain (optional)
  - [ ] Configure DNS
  - [ ] Set up SSL certificate
  - [ ] Update manifest.json with domain

---

### Story E5-S3: CI/CD Pipeline
**Priority**: P2 | **Est.**: 4h

- [ ] **T5.7** Create GitHub Actions workflow
  - [ ] Set up Rust toolchain
  - [ ] Install wasm-pack
  - [ ] Build WASM module
  - [ ] Run tests
  - [ ] Deploy to hosting

- [ ] **T5.8** Add branch protections
  - [ ] Require PR reviews
  - [ ] Require status checks
  - [ ] Prevent direct pushes to main

---

## Epic E6: Documentation

**Objective**: Create comprehensive documentation for users and developers.

**Acceptance Criteria**:
- [ ] README explains project and setup
- [ ] User guide covers basic usage
- [ ] API documentation is complete
- [ ] Architecture decisions are documented

---

### Story E6-S1: User Documentation
**Priority**: P1 | **Est.**: 4h

- [ ] **T6.1** Write README.md
  - [ ] Project description
  - [ ] Features list
  - [ ] Browser requirements
  - [ ] Quick start guide
  - [ ] Screenshots/GIFs

- [ ] **T6.2** Create USER_GUIDE.md
  - [ ] Step-by-step setup instructions
  - [ ] How to authenticate with Passkey
  - [ ] How to start/stop the node
  - [ ] Troubleshooting common issues
  - [ ] FAQ

- [ ] **T6.3** Add inline help
  - [ ] Tooltips for UI elements
  - [ ] Help modal with instructions
  - [ ] Link to full documentation

---

### Story E6-S2: Developer Documentation
**Priority**: P1 | **Est.**: 4h

- [ ] **T6.4** Write ARCHITECTURE.md
  - [ ] System overview
  - [ ] Component descriptions
  - [ ] Data flow diagrams
  - [ ] Technology choices rationale

- [ ] **T6.5** Document API
  - [ ] WASM function reference
  - [ ] JavaScript API reference
  - [ ] Type definitions
  - [ ] Example usage

- [ ] **T6.6** Write CONTRIBUTING.md
  - [ ] Development setup
  - [ ] Code style guidelines
  - [ ] Testing requirements
  - [ ] PR process

---

## Epic E7: Future Enhancements (Backlog)

**Objective**: Track features for future development beyond MVP.

### Story E7-S1: iroh-docs Integration
**Priority**: P2

- [ ] Integrate iroh-docs crate
- [ ] Implement namespace creation/opening
- [ ] Add document sync functionality
- [ ] Create document viewer UI
- [ ] Implement key prefix support (trust:/claim:/pointer:)

### Story E7-S2: UCAN Delegation
**Priority**: P2

- [ ] Add UCAN library dependency
- [ ] Implement capability token creation
- [ ] Add delegation flow UI
- [ ] Implement token validation
- [ ] Add token revocation

### Story E7-S3: Identity Key Separation
**Priority**: P2

- [ ] Derive identity key (secp256k1)
- [ ] Implement signing operations
- [ ] Add W3C VC support
- [ ] Create identity management UI

### Story E7-S4: Enhanced UI
**Priority**: P2

- [ ] Add peer list display
- [ ] Show connection statistics
- [ ] Add document management UI
- [ ] Implement dark/light mode toggle
- [ ] Add animations and transitions

### Story E7-S5: Background Sync
**Priority**: P3

- [ ] Implement Service Worker
- [ ] Add push notification support
- [ ] Enable background sync
- [ ] Add offline support

### Story E7-S6: Mobile Native Wrapper
**Priority**: P3

- [ ] Evaluate Capacitor vs Tauri
- [ ] Create native app wrapper
- [ ] Enable background execution
- [ ] Add native notifications

---

## Appendix A: Task Dependencies

```
E1 (Foundation)
    └── E2 (Rust Core)
            └── E3 (Web UI)
                    ├── E4 (Testing)
                    ├── E5 (Deployment)
                    └── E6 (Documentation)
```

## Appendix B: Definition of Done

### For Tasks
- [ ] Code written and compiles
- [ ] Self-reviewed
- [ ] Committed with descriptive message

### For Stories
- [ ] All subtasks complete
- [ ] Acceptance criteria met
- [ ] Manual testing performed
- [ ] Documentation updated

### For Epics
- [ ] All stories complete
- [ ] Integration testing passed
- [ ] Documentation complete
- [ ] Deployed to production
