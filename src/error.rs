//! Error types for the Iroh Wallet WASM library.

use wasm_bindgen::prelude::*;

/// Error type for wallet operations.
/// 
/// This is a simplified error type for WASM compatibility.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WalletError {
    message: String,
    code: String,
}

#[wasm_bindgen]
impl WalletError {
    /// Create a new error with a message and code.
    #[wasm_bindgen(constructor)]
    pub fn new(message: String, code: String) -> Self {
        Self { message, code }
    }
    
    /// Get the error message.
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
    
    /// Get the error code.
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.code.clone()
    }
}

impl WalletError {
    /// Create a key derivation error.
    pub fn key_derivation<T: Into<String>>(msg: T) -> Self {
        Self {
            message: msg.into(),
            code: "KeyDerivation".to_string(),
        }
    }
    
    /// Create an invalid input error.
    pub fn invalid_input<T: Into<String>, U: Into<String>>(expected: T, actual: U) -> Self {
        Self {
            message: format!("Expected {}, got {}", expected.into(), actual.into()),
            code: "InvalidInput".to_string(),
        }
    }
    
    /// Create a node initialization error.
    pub fn node_init_failed<T: Into<String>>(msg: T) -> Self {
        Self {
            message: msg.into(),
            code: "NodeInitFailed".to_string(),
        }
    }
    
    /// Create a connection failed error.
    pub fn connection_failed<T: Into<String>>(msg: T) -> Self {
        Self {
            message: msg.into(),
            code: "ConnectionFailed".to_string(),
        }
    }
    
    /// Create an invalid address error.
    pub fn invalid_address<T: Into<String>>(msg: T) -> Self {
        Self {
            message: msg.into(),
            code: "InvalidAddress".to_string(),
        }
    }
    
    /// Create a not initialized error.
    pub fn not_initialized() -> Self {
        Self {
            message: "Node not initialized".to_string(),
            code: "NotInitialized".to_string(),
        }
    }
    
    /// Create a timeout error.
    pub fn timeout() -> Self {
        Self {
            message: "Operation timed out".to_string(),
            code: "Timeout".to_string(),
        }
    }
    
    /// Create an internal error.
    pub fn internal<T: Into<String>>(msg: T) -> Self {
        Self {
            message: msg.into(),
            code: "Internal".to_string(),
        }
    }
}

impl std::fmt::Display for WalletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for WalletError {}

/// Convert WalletError to JsValue for WASM error handling.
/// Note: We don't implement From directly due to wasm_bindgen conflicts.
pub fn error_to_jsvalue(err: WalletError) -> JsValue {
    JsValue::from_str(&err.to_string())
}

/// Result type alias for wallet operations.
pub type Result<T> = std::result::Result<T, WalletError>;
