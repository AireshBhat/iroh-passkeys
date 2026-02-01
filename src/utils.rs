//! Utility functions for the Iroh Wallet WASM library.

use wasm_bindgen::prelude::*;

/// Initialize logging for the WASM module.
///
/// This function sets up console logging when available.
pub fn init_logging() {
    // Logging is initialized via wasm_bindgen start function
    // Additional logging setup can be added here
}

/// Log a message to the browser console (WASM only).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Log a message (native version).
#[cfg(not(target_arch = "wasm32"))]
fn log(s: &str) {
    println!("{}", s);
}

/// Log an info message.
pub fn console_log(msg: &str) {
    log(msg);
}

/// Format a node ID for display (short form).
pub fn format_node_id_short(node_id: &str) -> String {
    if node_id.len() > 16 {
        format!("{}...{}", &node_id[..8], &node_id[node_id.len() - 8..])
    } else {
        node_id.to_string()
    }
}

#[cfg(test)]
mod tests {}
