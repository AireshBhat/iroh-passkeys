//! Iroh Wallet WASM - Personal Iroh Node for Mobile Browsers
//!
//! This library provides WASM bindings for running an Iroh P2P node
//! within a web browser, with keys derived from WebAuthn PRF (Passkeys).

// Use wee_alloc for smaller bundle size when enabled
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use wasm_bindgen::prelude::*;

// Module declarations
pub mod crypto;
pub mod error;
pub mod keys;
pub mod node;
pub mod utils;

// Re-export main types for JavaScript
pub use error::WalletError;
pub use keys::{derive_transport_key, generate_transport_key};
pub use node::IrohNode;

/// Initialize the WASM module.
///
/// This function should be called once when the module is loaded.
/// It sets up panic hooks and logging for better debugging.
#[wasm_bindgen(start)]
pub fn start() {
    // Set up panic hook for better error messages in console
    #[cfg(all(feature = "console_error_panic_hook", target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    // Initialize logging if available
    #[cfg(target_arch = "wasm32")]
    utils::init_logging();
}

/// Get the library version.
#[wasm_bindgen(js_name = "version")]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Check if WebAssembly is properly supported and initialized.
#[wasm_bindgen(js_name = "isReady")]
pub fn is_ready() -> bool {
    true
}
