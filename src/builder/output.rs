use crate::bindings::ntwk::theater::message_server_host::send;
use crate::bindings::ntwk::theater::runtime::log;
use crate::state::BuildOutput;
use serde_json::json;

/// Trait for sending build results
pub trait BuildResultSender {
    fn send_build_results(&self, callback_address: &str, build_output: &BuildOutput);
}

/// Implementation of BuildResultSender for the default implementation
pub struct DefaultBuildResultSender;

impl BuildResultSender for DefaultBuildResultSender {
    /// Function to send build results to the callback address
    fn send_build_results(&self, callback_address: &str, build_output: &BuildOutput) {
        log(&format!("Sending build results to {}", callback_address));

        // Create message
        let message = json!({
            "action": "build_result",
            "success": build_output.success,
            "wasm_hash": build_output.wasm_hash,
            "logs": build_output.build_logs,
            "error": build_output.error,
            "stdout": build_output.stdout,
            "stderr": build_output.stderr
        });

        // Send message
        match serde_json::to_vec(&message) {
            Ok(message_bytes) => match send(&callback_address.to_string(), &message_bytes) {
                Ok(_) => log("Successfully sent build results"),
                Err(e) => log(&format!("Failed to send build results: {}", e)),
            },
            Err(e) => log(&format!("Failed to serialize build results: {}", e)),
        }
    }
}
