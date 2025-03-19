# Change Request: Build Actor Channel Streaming Implementation

## Overview

This change request outlines the modifications needed for the build actor to support channel-based streaming of events during the build process. The build actor will provide real-time updates about extraction progress, compilation status, and build outputs to improve visibility into the build process.

## Implementation Details

### 1. State Structure Updates

Update the `BuildState` struct in `src/lib.rs` to support channel tracking:

```

### 6. Directory Processing with Streaming

Add a version of the directory processing function that streams events:

```rust
// Process directory with streaming events
fn process_directory_with_streaming(
    fs_hash: &str,
    path: &str,
    entries: &[DirectoryEntry],
    store_id: &str,
    state: &mut BuildState,
) -> Result<(), String> {
    for entry in entries {
        let entry_path = if path == "/" {
            format!("/{}", entry.name)
        } else {
            format!("{}/{}", path, entry.name)
        };

        log(&format!("Processing entry: {}", entry_path));
        
        // Stream file processing event
        if let Err(e) = state.stream_event("output.log", json!({
            "message": format!("Processing {}: {}", 
                if entry.entry_type == "directory" { "directory" } else { "file" }, 
                entry_path)
        })) {
            log(&format!("Failed to stream event: {}", e));
        }

        if entry.entry_type == "directory" {
            // Stream directory creation event
            if let Err(e) = state.stream_event("build.creating_directory", json!({
                "path": entry_path
            })) {
                log(&format!("Failed to stream event: {}", e));
            }
            
            // Create directory
            if let Err(e) = filesystem::create_dir(&format!(".{}", entry_path)) {
                // Stream error event
                if let Err(stream_err) = state.stream_event("build.error", json!({
                    "message": format!("Failed to create directory {}: {}", entry_path, e),
                    "path": entry_path,
                    "error": e
                })) {
                    log(&format!("Failed to stream event: {}", stream_err));
                }
                
                log(&format!("Failed to create directory {}: {}", entry_path, e));
                return Err(format!("Failed to create directory {}: {}", entry_path, e));
            }

            // List directory contents
            match list_directory(fs_hash, &entry_path, store_id) {
                Ok(sub_entries) => {
                    // Stream subdirectory info
                    if let Err(e) = state.stream_event("output.log", json!({
                        "message": format!("Found {} entries in {}", sub_entries.len(), entry_path)
                    })) {
                        log(&format!("Failed to stream event: {}", e));
                    }
                    
                    // Process subdirectory with streaming
                    if let Err(e) = process_directory_with_streaming(fs_hash, &entry_path, &sub_entries, store_id, state) {
                        return Err(e);
                    }
                }
                Err(e) => {
                    // Stream error event
                    if let Err(stream_err) = state.stream_event("build.error", json!({
                        "message": format!("Failed to list directory {}: {}", entry_path, e),
                        "path": entry_path,
                        "error": e
                    })) {
                        log(&format!("Failed to stream event: {}", stream_err));
                    }
                    
                    log(&format!("Failed to list directory {}: {}", entry_path, e));
                    return Err(format!("Failed to list directory {}: {}", entry_path, e));
                }
            }
        } else {
            // Stream file extraction event
            if let Err(e) = state.stream_event("build.extracting_file", json!({
                "path": entry_path
            })) {
                log(&format!("Failed to stream event: {}", e));
            }
            
            // Read file content
            match read_file(fs_hash, &entry_path, store_id) {
                Ok(content) => {
                    // Stream file size info
                    if let Err(e) = state.stream_event("output.log", json!({
                        "message": format!("Extracted file: {} ({} bytes)", entry_path, content.len())
                    })) {
                        log(&format!("Failed to stream event: {}", e));
                    }
                    
                    // Write file to local filesystem
                    if let Err(e) = filesystem::write_file(&format!(".{}", entry_path), &content) {
                        // Stream error event
                        if let Err(stream_err) = state.stream_event("build.error", json!({
                            "message": format!("Failed to write file {}: {}", entry_path, e),
                            "path": entry_path,
                            "error": e
                        })) {
                            log(&format!("Failed to stream event: {}", stream_err));
                        }
                        
                        log(&format!("Failed to write file {}: {}", entry_path, e));
                        return Err(format!("Failed to write file {}: {}", entry_path, e));
                    }
                }
                Err(e) => {
                    // Stream error event
                    if let Err(stream_err) = state.stream_event("build.error", json!({
                        "message": format!("Failed to read file {}: {}", entry_path, e),
                        "path": entry_path,
                        "error": e
                    })) {
                        log(&format!("Failed to stream event: {}", stream_err));
                    }
                    
                    log(&format!("Failed to read file {}: {}", entry_path, e));
                    return Err(format!("Failed to read file {}: {}", entry_path, e));
                }
            }
        }
    }

    Ok(())
}
```

### 7. Build Execution with Streaming

Add a version of the build execution function that streams events:

```rust
// Execute build with streaming events
fn execute_build_with_streaming(
    build_store_id: &str,
    state: &mut BuildState,
) -> Result<BuildOutput, String> {
    log("Executing build command with streaming");
    
    // Stream build command event
    if let Err(e) = state.stream_event("build.compiling", json!({
        "message": "Compiling with cargo component build",
        "target": "wasm32-unknown-unknown",
        "release": true
    })) {
        log(&format!("Failed to stream event: {}", e));
    }

    // Execute the nix build command
    match filesystem::execute_command(
        ".",
        "nix",
        &[
            "develop".to_string(),
            "--command".to_string(),
            "bash".to_string(),
            "-c".to_string(),
            "cargo component build --target wasm32-unknown-unknown --release".to_string(),
        ],
    ) {
        Ok(stdout) => {
            log(&format!("Build command executed, stdout: {}", stdout));
            
            // Stream compilation output
            let stdout_lines: Vec<&str> = stdout.lines().collect();
            for (i, line) in stdout_lines.iter().enumerate() {
                if !line.trim().is_empty() {
                    if let Err(e) = state.stream_event("output.stdout", json!({
                        "message": line.trim(),
                        "line": i
                    })) {
                        log(&format!("Failed to stream event: {}", e));
                    }
                    
                    // Add a small delay to avoid flooding the channel
                    if i % 10 == 0 {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }

            // Capture stderr (not directly available from host function)
            let stderr = "Stderr not available from host function".to_string();
            
            // Stream build completion event
            if let Err(e) = state.stream_event("build.linking", json!({
                "message": "Linking WebAssembly module"
            })) {
                log(&format!("Failed to stream event: {}", e));
            }

            // Check if target directory exists
            let target_dir_path = "target/wasm32-unknown-unknown/release";
            match filesystem::list_files(target_dir_path) {
                Ok(files) => {
                    log(&format!("Found {} files in target directory", files.len()));
                    
                    // Stream file list event
                    if let Err(e) = state.stream_event("output.log", json!({
                        "message": format!("Found {} files in target directory", files.len())
                    })) {
                        log(&format!("Failed to stream event: {}", e));
                    }

                    // Find .wasm file
                    let wasm_files: Vec<String> = files
                        .iter()
                        .filter(|f| f.ends_with(".wasm"))
                        .cloned()
                        .collect();

                    if let Some(wasm_file) = wasm_files.first() {
                        let wasm_path = format!("{}/{}", target_dir_path, wasm_file);
                        log(&format!("Found WASM file at {}", wasm_path));
                        
                        // Stream wasm file found event
                        if let Err(e) = state.stream_event("build.packaging", json!({
                            "message": format!("Found WASM file: {}", wasm_file),
                            "path": wasm_path
                        })) {
                            log(&format!("Failed to stream event: {}", e));
                        }

                        // Read WASM file to calculate hash
                        match filesystem::read_file(&wasm_path) {
                            Ok(wasm_bytes) => {
                                // Calculate hash (basic string hash for now)
                                let wasm_hash = format!("wasm-{}", wasm_bytes.len());
                                let wasm_size = wasm_bytes.len();

                                log(&format!(
                                    "Storing WASM file at {} in store {}",
                                    wasm_hash, build_store_id
                                ));
                                
                                // Stream storing event
                                if let Err(e) = state.stream_event("build.storing", json!({
                                    "message": format!("Storing WASM file ({} bytes)", wasm_size),
                                    "hash": wasm_hash,
                                    "size": wasm_size
                                })) {
                                    log(&format!("Failed to stream event: {}", e));
                                }
                                
                                store::store_at_label(build_store_id, "wasm", &wasm_bytes)
                                    .map_err(|e| format!("Failed to store WASM file: {}", e))?;

                                // Create build output
                                let build_output = BuildOutput {
                                    success: true,
                                    stdout,
                                    stderr,
                                    wasm_path: Some(wasm_path),
                                    wasm_hash: Some(wasm_hash),
                                    build_logs: vec!["Build completed successfully".to_string()],
                                    error: None,
                                };

                                Ok(build_output)
                            }
                            Err(e) => {
                                log(&format!("Failed to read WASM file: {}", e));
                                
                                // Stream error event
                                if let Err(stream_err) = state.stream_event("build.error", json!({
                                    "message": format!("Failed to read WASM file: {}", e),
                                    "path": wasm_path,
                                    "error": e
                                })) {
                                    log(&format!("Failed to stream event: {}", stream_err));
                                }

                                // Create build output with error
                                let build_output = BuildOutput {
                                    success: false,
                                    stdout,
                                    stderr,
                                    wasm_path: Some(wasm_path),
                                    wasm_hash: None,
                                    build_logs: vec!["Failed to read WASM file".to_string()],
                                    error: Some(format!("Failed to read WASM file: {}", e)),
                                };

                                Ok(build_output)
                            }
                        }
                    } else {
                        log("No WASM file found");
                        
                        // Stream error event
                        if let Err(stream_err) = state.stream_event("build.error", json!({
                            "message": "No WASM file found after build"
                        })) {
                            log(&format!("Failed to stream event: {}", stream_err));
                        }

                        // Create build output with error
                        let build_output = BuildOutput {
                            success: false,
                            stdout,
                            stderr,
                            wasm_path: None,
                            wasm_hash: None,
                            build_logs: vec!["No WASM file found".to_string()],
                            error: Some("No WASM file found".to_string()),
                        };

                        Ok(build_output)
                    }
                }
                Err(e) => {
                    log(&format!("Failed to list target directory: {}", e));
                    
                    // Stream error event
                    if let Err(stream_err) = state.stream_event("build.error", json!({
                        "message": format!("Failed to list target directory: {}", e),
                        "error": e
                    })) {
                        log(&format!("Failed to stream event: {}", stream_err));
                    }

                    // Create build output with error
                    let build_output = BuildOutput {
                        success: false,
                        stdout,
                        stderr,
                        wasm_path: None,
                        wasm_hash: None,
                        build_logs: vec!["Failed to list target directory".to_string()],
                        error: Some(format!("Failed to list target directory: {}", e)),
                    };

                    Ok(build_output)
                }
            }
        }
        Err(e) => {
            log(&format!("Failed to execute build command: {}", e));
            
            // Stream error event
            if let Err(stream_err) = state.stream_event("build.failed", json!({
                "message": format!("Failed to execute build command: {}", e),
                "error": e
            })) {
                log(&format!("Failed to stream event: {}", stream_err));
            }

            // Create build output with error
            let build_output = BuildOutput {
                success: false,
                stdout: String::new(),
                stderr: format!("Failed to execute build command: {}", e),
                wasm_path: None,
                wasm_hash: None,
                build_logs: vec!["Build command failed".to_string()],
                error: Some(format!("Failed to execute build command: {}", e)),
            };

            Ok(build_output)
        }
    }
}
```

## Testing Requirements

1. Test channel establishment with the manager actor
2. Test streaming events during the build process
3. Test streaming of file extraction events
4. Test streaming of compilation output
5. Test streaming of build success/failure events
6. Test error handling and error event streaming

## Acceptance Criteria

1. The build actor accepts channel connections from the manager
2. Real-time events are streamed during all stages of the build process:
   - File extraction from content store
   - Directory creation
   - Build environment setup
   - Compilation progress
   - Linking and packaging
   - Build completion or failure
3. Events follow the standardized event format with all required fields
4. Build output is streamed in real-time (stdout/stderr)
5. Channel resources are properly cleaned up when operations complete
6. The implementation maintains backward compatibility with non-streaming clients
7. Error conditions are properly reported as events on the channel
8. The channel is automatically closed when the build completes or fails
rust
// Update BuildState struct
struct BuildState {
    // Existing fields
    store_id: String,
    build_store_id: String,
    fs_hash: String,
    status: BuildStatus,
    build_output: Option<BuildOutput>,
    
    // New fields for streaming
    active_channel: Option<String>,
    operation_id: Option<String>,
    event_sequence: u32,
}
```

### 2. Event Streaming Function

Add a method to the `BuildState` struct to stream events:

```rust
// Add to BuildState struct
impl BuildState {
    // Stream an event on the active channel
    fn stream_event(&mut self, event_type: &str, content: serde_json::Value) -> Result<(), String> {
        if let (Some(channel_id), Some(operation_id)) = (&self.active_channel, &self.operation_id) {
            let event = json!({
                "event_type": event_type,
                "source": "build-actor",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "sequence": self.event_sequence,
                "operation_id": operation_id,
                "content": content
            });
            
            self.event_sequence += 1;
            
            let event_bytes = serde_json::to_vec(&event).map_err(|e| e.to_string())?;
            
            ntwk_theater_message_server_host_send_on_channel(channel_id, &event_bytes)
                .map_err(|e| format!("Failed to send event: {}", e))?;
        }
        
        Ok(())
    }
}
```

### 3. Channel Open Handler

Add a handler to accept channel connections:

```rust
// Add to the MessageServerClient implementation
fn handle_channel_open(
    state_bytes: Option<Vec<u8>>,
    params: Vec<u8>,
) -> Result<(Option<Vec<u8>>, bool, Option<Vec<u8>>), String> {
    log("Build actor: Channel open request received");
    
    // Parse state
    let mut state: BuildState = match state_bytes {
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Failed to parse state: {}", e))?,
        None => return Ok((None, false, None)),
    };
    
    // Parse the initial message
    let event: serde_json::Value = serde_json::from_slice(&params)
        .map_err(|e| format!("Failed to parse channel open message: {}", e))?;
    
    // Check if this is a build request channel
    if let Some(event_type) = event.get("event_type").and_then(|v| v.as_str()) {
        if event_type == "operation.started" {
            // Extract operation ID
            let operation_id = event.get("operation_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing operation_id in channel open request")?
                .to_string();
            
            // Accept the channel
            state.operation_id = Some(operation_id.clone());
            state.event_sequence = 0;
            
            // Prepare an acknowledgment response
            let response = json!({
                "event_type": "operation.acknowledged",
                "source": "build-actor",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "sequence": state.event_sequence,
                "operation_id": operation_id,
                "content": {
                    "status": "ready",
                    "message": "Build actor ready to process build request"
                }
            });
            
            state.event_sequence += 1;
            
            let response_bytes = serde_json::to_vec(&response).map_err(|e| e.to_string())?;
            let updated_state = serde_json::to_vec(&state).map_err(|e| e.to_string())?;
            
            return Ok((Some(updated_state), true, Some(response_bytes)));
        }
    }
    
    // Reject other channel types
    let updated_state = serde_json::to_vec(&state).map_err(|e| e.to_string())?;
    Ok((Some(updated_state), false, None))
}
```

### 4. Channel Message Handler

Add a handler to process messages received on channels:

```rust
fn handle_channel_message(
    state_bytes: Option<Vec<u8>>,
    channel_id: String,
    msg: Vec<u8>,
) -> Result<Option<Vec<u8>>, String> {
    log(&format!("Build actor: Received message on channel {}", channel_id));
    
    // Parse state
    let mut state: BuildState = match state_bytes {
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Failed to parse state: {}", e))?,
        None => return Ok((None,)),
    };
    
    // Store the channel ID
    state.active_channel = Some(channel_id.clone());
    
    // Parse the message
    let event: serde_json::Value = serde_json::from_slice(&msg)
        .map_err(|e| format!("Failed to parse channel message: {}", e))?;
    
    // Handle build request
    if let Some(event_type) = event.get("event_type").and_then(|v| v.as_str()) {
        if event_type == "build.request" {
            // Extract build parameters from the content
            let content = event.get("content").ok_or("Missing content in build request")?;
            
            // Extract required fields
            let fs_hash = content.get("fs_hash")
                .and_then(|v| v.as_str())
                .ok_or("Missing fs_hash in build request")?;
                
            let store_id = content.get("store_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing store_id in build request")?;
                
            let build_store_id = content.get("build_store_id")
                .and_then(|v| v.as_str())
                .ok_or("Missing build_store_id in build request")?;
            
            // Update state with build parameters
            state.fs_hash = fs_hash.to_string();
            state.store_id = store_id.to_string();
            state.build_store_id = build_store_id.to_string();
            state.status = BuildStatus::NotStarted;
            
            // Start the build process
            if let Err(e) = state.stream_event("build.started", json!({
                "message": "Build process starting",
                "fs_hash": fs_hash
            })) {
                log(&format!("Failed to stream event: {}", e));
            }
            
            // Start the build process
            let build_output = start_build_with_streaming(&mut state);
            
            // Update state with build result
            state.build_output = Some(build_output.clone());
            state.status = if build_output.success {
                BuildStatus::Completed
            } else {
                BuildStatus::Failed
            };
            
            // Stream final event
            let status = if build_output.success { "success" } else { "failed" };
            if let Err(e) = state.stream_event("operation.completed", json!({
                "message": format!("Build operation {} ({})", status, build_output.error.unwrap_or_default()),
                "success": build_output.success
            })) {
                log(&format!("Failed to stream event: {}", e));
            }
        }
    }
    
    let updated_state = serde_json::to_vec(&state).map_err(|e| e.to_string())?;
    Ok(Some(updated_state))
}

fn handle_channel_close(
    state_bytes: Option<Vec<u8>>,
    channel_id: String,
) -> Result<Option<Vec<u8>>, String> {
    log(&format!("Build actor: Channel {} closed", channel_id));
    
    // Parse state
    let mut state: BuildState = match state_bytes {
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Failed to parse state: {}", e))?,
        None => return Ok((None,)),
    };
    
    // Clear the active channel if it matches
    if state.active_channel.as_ref().map_or(false, |ac| ac == &channel_id) {
        state.active_channel = None;
        state.operation_id = None;
    }
    
    let updated_state = serde_json::to_vec(&state).map_err(|e| e.to_string())?;
    Ok(Some(updated_state))
}
```

### 5. Modified Build Process

Create a new version of the build process function that streams events:

```rust
// New function to start build with streaming
fn start_build_with_streaming(state: &mut BuildState) -> BuildOutput {
    log("Starting build process with streaming");
    
    // First update state to reflect that we're starting extraction
    let mut updated_state = state.clone();
    updated_state.status = BuildStatus::Extracting;
    
    // Stream extracting event
    if let Err(e) = state.stream_event("build.extracting", json!({
        "message": "Extracting code from content store"
    })) {
        log(&format!("Failed to stream event: {}", e));
    }
    
    // Begin extracting files from virtual filesystem
    let fs_hash = &state.fs_hash;

    // Directory listing from root
    match list_directory(fs_hash, "/", &state.store_id) {
        Ok(entries) => {
            log(&format!("Found {} entries at root", entries.len()));
            
            // Stream root directory info
            if let Err(e) = state.stream_event("output.log", json!({
                "message": format!("Found {} entries at root", entries.len())
            })) {
                log(&format!("Failed to stream event: {}", e));
            }

            // Process entries recursively with streaming
            match process_directory_with_streaming(fs_hash, "/", &entries, &state.store_id, state) {
                Ok(_) => {
                    log("Successfully extracted all files from virtual filesystem");
                    
                    // Stream extraction complete event
                    if let Err(e) = state.stream_event("build.extraction_complete", json!({
                        "message": "Successfully extracted all files from virtual filesystem"
                    })) {
                        log(&format!("Failed to stream event: {}", e));
                    }

                    // Update state to reflect we're now building
                    let mut building_state = updated_state.clone();
                    building_state.status = BuildStatus::Building;
                    
                    // Stream build starting event
                    if let Err(e) = state.stream_event("build.environment_setup", json!({
                        "message": "Setting up build environment"
                    })) {
                        log(&format!("Failed to stream event: {}", e));
                    }

                    // Start the build process with streaming
                    match execute_build_with_streaming(&state.build_store_id, state) {
                        Ok(build_output) => {
                            log(&format!(
                                "Build completed with success={}",
                                build_output.success
                            ));
                            
                            // Stream build completed event
                            let status_msg = if build_output.success {
                                "Build completed successfully"
                            } else {
                                "Build failed"
                            };
                            
                            if let Err(e) = state.stream_event(
                                if build_output.success { "build.completed" } else { "build.failed" },
                                json!({
                                    "message": status_msg,
                                    "wasm_path": build_output.wasm_path,
                                    "success": build_output.success
                                })
                            ) {
                                log(&format!("Failed to stream event: {}", e));
                            }

                            // Update state with build output
                            let mut final_state = building_state.clone();
                            final_state.build_output = Some(build_output.clone());
                            final_state.status = if build_output.success {
                                BuildStatus::Completed
                            } else {
                                BuildStatus::Failed
                            };

                            build_output
                        }
                        Err(e) => {
                            log(&format!("Build failed: {}", e));
                            
                            // Stream build failed event
                            if let Err(stream_err) = state.stream_event("build.failed", json!({
                                "message": format!("Build failed: {}", e),
                                "error": e
                            })) {
                                log(&format!("Failed to stream event: {}", stream_err));
                            }

                            // Update state to reflect failure
                            let mut failed_state = building_state.clone();
                            failed_state.status = BuildStatus::Failed;
                            failed_state.build_output = Some(BuildOutput {
                                success: false,
                                stdout: String::new(),
                                stderr: String::new(),
                                wasm_path: None,
                                wasm_hash: None,
                                build_logs: vec![],
                                error: Some(e.clone()),
                            });

                            BuildOutput {
                                success: false,
                                stdout: String::new(),
                                stderr: String::new(),
                                wasm_path: None,
                                wasm_hash: None,
                                build_logs: vec![],
                                error: Some(e),
                            }
                        }
                    }
                }
                Err(e) => {
                    log(&format!("Failed to process directories: {}", e));
                    
                    // Stream error event
                    if let Err(stream_err) = state.stream_event("build.extraction_failed", json!({
                        "message": format!("Failed to process directories: {}", e),
                        "error": e
                    })) {
                        log(&format!("Failed to stream event: {}", stream_err));
                    }
                    
                    BuildOutput {
                        success: false,
                        stdout: String::new(),
                        stderr: String::new(),
                        wasm_path: None,
                        wasm_hash: None,
                        build_logs: vec![],
                        error: Some(e),
                    }
                }
            }
        }
        Err(e) => {
            log(&format!("Failed to list root directory: {}", e));
            
            // Stream error event
            if let Err(stream_err) = state.stream_event("build.extraction_failed", json!({
                "message": format!("Failed to list root directory: {}", e),
                "error": e
            })) {
                log(&format!("Failed to stream event: {}", stream_err));
            }
            
            BuildOutput {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                wasm_path: None,
                wasm_hash: None,
                build_logs: vec![],
                error: Some(e),
            }
        }
    }
}
