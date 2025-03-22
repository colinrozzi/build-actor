use crate::bindings::exports::ntwk::theater::message_server_client::{self, ChannelAccept, Json};
use crate::bindings::ntwk::theater::message_server_host::send_on_channel;
use crate::bindings::ntwk::theater::runtime::log;
use crate::builder::process::BuildProcess;
use crate::messaging::messages::{BuildMessage, LogLevel};
use crate::state::BuildState;
use serde_json::Value;

/// Message handling implementation
pub struct MessageHandler;

impl MessageHandler {
    /// Handle initialization
    pub fn init(init_data: Option<Json>, _params: (String,)) -> Result<(Option<Json>,), String> {
        log("build-actor: Initializing");
        log(&format!("Initialization data: {:?}", init_data));
        Ok((init_data,))
    }

    /// Handle send messages
    pub fn handle_send(state: Option<Json>, _params: (Json,)) -> Result<(Option<Json>,), String> {
        // Return state unchanged
        Ok((state,))
    }

    /// Handle request messages
    pub fn handle_request(
        state_bytes: Option<Json>,
        _params: (Json,),
    ) -> Result<(Option<Json>, (Json,)), String> {
        log("build-actor: Received request");
        Ok((state_bytes, (vec![],)))
    }

    /// Handle channel open
    pub fn handle_channel_open(
        state: Option<Json>,
        _params: (Json,),
    ) -> Result<(Option<Json>, (ChannelAccept,)), String> {
        log("build-actor: Channel connection request received");

        // Always accept channel connections
        Ok((
            state, // Pass through any existing state unchanged
            (ChannelAccept {
                accepted: true,
                message: None,
            },),
        ))
    }

    /// Handle channel close
    pub fn handle_channel_close(
        state: Option<Json>,
        _params: (String,),
    ) -> Result<(Option<Json>,), String> {
        Ok((state,))
    }

    /// Handle channel message
    pub fn handle_channel_message(
        state_bytes: Option<Json>,
        params: (String, Json),
    ) -> Result<(Option<Json>,), String> {
        let (channel_id, message_data) = params;
        log(&format!(
            "build-actor: Received channel message on {}",
            channel_id
        ));

        // Parse message
        let message = match serde_json::from_slice::<Value>(&message_data) {
            Ok(msg) => msg,
            Err(e) => return Err(format!("Failed to parse message: {}", e)),
        };

        log(&format!("Received message: {:?}", message));

        // Get command type
        let command = match message.get("command").and_then(|v| v.as_str()) {
            Some(cmd) => cmd,
            None => return Err("Missing 'command' field".to_string()),
        };

        log(&format!("Received command: {}", command));

        match command {
            "start_build" => {
                // Extract build parameters
                let fs_hash = match message.get("fs_hash").and_then(|v| v.as_str()) {
                    Some(hash) => hash,
                    None => return Err("Missing 'fs_hash' parameter".to_string()),
                };

                let store_id = match message.get("store_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return Err("Missing 'store_id' parameter".to_string()),
                };

                let build_store_id = match message.get("build_store_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return Err("Missing 'build_store_id' parameter".to_string()),
                };

                // Create a new BuildState with the provided parameters
                let mut build_state = BuildState::new(
                    store_id.to_string(),
                    fs_hash.to_string(),
                    build_store_id.to_string(),
                );

                // Store the channel ID for sending progress updates
                build_state.channel_id = Some(channel_id.clone());

                // Create build process and start it
                let mut build_process = BuildProcess::new(build_state);
                let result = build_process.start(); // This now sends progress over channel

                // After completion, store the final state
                let updated_state = build_process.state();
                let updated_state_bytes = match serde_json::to_vec(updated_state) {
                    Ok(bytes) => bytes,
                    Err(e) => return Err(format!("Failed to serialize state: {}", e)),
                };

                Ok((Some(updated_state_bytes),))
            }
            "check_status" => {
                // If we have a state, return current build status
                if let Some(state_data) = state_bytes {
                    match serde_json::from_slice::<BuildState>(&state_data) {
                        Ok(state) => {
                            // Send current status as a message
                            let status_msg = BuildMessage::Progress {
                                status: state.status.clone(),
                                description: format!("Current build status: {:?}", state.status),
                                percent_complete: None,
                            };

                            if let Ok(msg_json) = serde_json::to_vec(&status_msg) {
                                if let Err(e) = send_on_channel(&channel_id, &msg_json) {
                                    log(&format!("Failed to send status message: {}", e));
                                }
                            }

                            Ok((Some(state_data),))
                        }
                        Err(e) => Err(format!("Failed to deserialize state: {}", e)),
                    }
                } else {
                    // No state - send error message
                    let error_msg = BuildMessage::Log {
                        level: LogLevel::Error,
                        message: "No build state available".to_string(),
                    };

                    if let Ok(msg_json) = serde_json::to_vec(&error_msg) {
                        if let Err(e) = send_on_channel(&channel_id, &msg_json) {
                            log(&format!("Failed to send error message: {}", e));
                        }
                    }

                    Ok((None,))
                }
            }
            _ => Err(format!("Unknown command: {}", command)),
        }
    }
}
