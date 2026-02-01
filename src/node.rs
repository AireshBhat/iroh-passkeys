//! Iroh node management module.
//!
//! This module provides the `IrohNode` struct for managing an Iroh
//! P2P endpoint within a web browser via WASM.
//!
//! Note: Full Iroh integration will be implemented in later tasks.
//!       This is a placeholder structure for the initial setup.

use ed25519_dalek::SigningKey;
use wasm_bindgen::prelude::*;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Iroh node instance for P2P networking.
///
/// This struct manages the lifecycle of an Iroh endpoint, including
/// initialization, connection management, and graceful shutdown.
#[wasm_bindgen]
#[derive(ZeroizeOnDrop)]
pub struct IrohNode {
    /// The node ID (public key) as a string.
    #[zeroize(skip)]
    node_id: String,

    /// Whether the node is currently connected.
    #[zeroize(skip)]
    connected: bool,

    /// Transport key (kept for reconnection purposes).
    transport_key: [u8; 32],
}

#[wasm_bindgen]
impl IrohNode {
    /// Create a new Iroh node with the given transport key.
    ///
    /// # Arguments
    ///
    /// * `transport_key` - A 32-byte Ed25519 seed for node identity.
    ///
    /// # Returns
    ///
    /// Returns a new `IrohNode` instance initialized and connected
    /// to the Iroh relay network.
    #[wasm_bindgen(constructor)]
    pub async fn new(transport_key: Box<[u8]>) -> std::result::Result<IrohNode, JsValue> {
        // Validate key length
        if transport_key.len() != 32 {
            return Err(JsValue::from_str(&format!(
                "Invalid transport key length: expected 32, got {}",
                transport_key.len()
            )));
        }

        // Convert to fixed-size array
        let key_array: [u8; 32] = transport_key
            .as_ref()
            .try_into()
            .map_err(|_| JsValue::from_str("Failed to convert transport key"))?;

        // Derive the node ID from the transport key
        // The node ID is the Ed25519 public key (verifying key) derived from
        // the 32-byte secret seed (transport key)
        let signing_key = SigningKey::from_bytes(&key_array);
        let verifying_key = signing_key.verifying_key();
        let node_id = zbase32::encode_full_bytes(verifying_key.as_bytes());

        Ok(IrohNode {
            node_id,
            connected: false, // Will be set to true when endpoint binds
            transport_key: key_array,
        })
    }

    /// Get the node ID (public key) as a string.
    #[wasm_bindgen(getter)]
    pub fn node_id(&self) -> String {
        self.node_id.clone()
    }

    /// Check if the node is currently connected to the relay.
    #[wasm_bindgen(js_name = "isConnected")]
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Shut down the node gracefully.
    ///
    /// This closes all connections, unregisters from discovery,
    /// and zeroizes sensitive key material.
    pub async fn shutdown(&mut self) -> std::result::Result<(), JsValue> {
        // Zeroize the transport key
        self.transport_key.zeroize();
        self.connected = false;

        Ok(())
    }

    /// Connect to a remote node.
    ///
    /// # Arguments
    ///
    /// * `node_addr` - The node address to connect to (as a string).
    ///
    /// # Returns
    ///
    /// Returns a connection handle or an error.
    #[wasm_bindgen(js_name = "connect")]
    pub async fn connect(&self, _node_addr: String) -> std::result::Result<JsValue, JsValue> {
        if !self.connected {
            return Err(JsValue::from_str("Node is not connected"));
        }

        // TODO: Implement actual connection logic in later tasks
        Err(JsValue::from_str("Not implemented"))
    }
}

impl IrohNode {
    /// Get the transport key (for internal use only).
    pub fn transport_key(&self) -> &[u8; 32] {
        &self.transport_key
    }
}

#[cfg(test)]
mod tests {}
