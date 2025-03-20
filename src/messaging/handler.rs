use crate::bindings::exports::ntwk::theater::message_server_client::{self, Json, ChannelAccept};
use crate::bindings::ntwk::theater::runtime::log;
use crate::builder::process::BuildProcess;
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
        params: (Json,),
    ) -> Result<(Option<Json>, (Json,)), String> {
        log("build-actor: Received request");
        let req = params.0;
        match serde_json::from_slice::<Value>(&req) {
            Ok(config) => {
                // Extract required parameters
                let fs_hash = match config.get("fs_hash").and_then(|v| v.as_str()) {
                    Some(hash) => hash.to_string(),
                    None => return Err("Missing required parameter 'fs_hash'".to_string()),
                };

                let store_id = match config.get("store_id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => return Err("Missing required parameter 'store_id'".to_string()),
                };

                // Create initial state
                let state = BuildState::new(store_id, fs_hash);

                // Start the build process
                let mut build_process = BuildProcess::new(state);
                let result = build_process.start();

                // Serialize the state
                let updated_state = build_process.state();
                match serde_json::to_vec(updated_state) {
                    Ok(state_bytes) => {
                        log("build-actor: Build process completed");
                        Ok((Some(state_bytes), (serde_json::to_vec(&result).unwrap(),)))
                    }
                    Err(e) => Err(format!("Failed to serialize state: {}", e)),
                }
            }
            Err(e) => Err(format!("Failed to parse initialization data: {}", e)),
        }
    }

    /// Handle channel open
    pub fn handle_channel_open(
        state: Option<Json>,
        _params: (Json,),
    ) -> Result<(Option<Json>, (ChannelAccept,)), String> {
        Ok((
            state,
            (
                ChannelAccept {
                    accepted: true,
                    message: None,
                },
            ),
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
        state: Option<Json>,
        params: (String, Json),
    ) -> Result<(Option<Json>,), String> {
        log("build-actor: Received channel message");
        Ok((state,))
    }
}
