mod bindings;

use bindings::exports::ntwk::theater::actor::Guest;
use bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClient;
use bindings::ntwk::theater::filesystem;
use bindings::ntwk::theater::message_server_host::send;
use bindings::ntwk::theater::runtime::log;
use bindings::ntwk::theater::store::{self, ContentRef};
use bindings::ntwk::theater::types::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct State {}

/// State structure for the build actor
#[derive(Debug, Serialize, Deserialize, Clone)]
struct BuildState {
    store_id: String,

    // Content reference to filesystem root in the Theater runtime store
    fs_hash: String,

    // Build status
    status: BuildStatus,

    // Build output information
    build_output: Option<BuildOutput>,
}

/// Status of the build process
#[derive(Debug, Serialize, Deserialize, Clone)]
enum BuildStatus {
    NotStarted,
    Extracting,
    Building,
    Completed,
    Failed,
}

/// Structure to hold build result information
#[derive(Debug, Serialize, Deserialize, Clone)]
struct BuildOutput {
    success: bool,
    stdout: String,
    stderr: String,
    wasm_path: Option<String>,
    wasm_hash: Option<String>,
    build_logs: Vec<String>,
    error: Option<String>,
}

/// Types of filesystem nodes
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum NodeType {
    File,
    Directory,
}

/// Represents a filesystem node (file or directory)
#[derive(Debug, Serialize, Deserialize, Clone)]
struct FSNode {
    /// For directories: mapping of child names to hash
    entries: Option<HashMap<String, String>>,
    /// For files: content data
    content: Option<Vec<u8>>,
    /// Type of node
    node_type: NodeType,
}

/// Structure representing a directory entry
#[derive(Debug, Serialize, Deserialize, Clone)]
struct DirectoryEntry {
    name: String,
    #[serde(rename = "type")]
    entry_type: String,
    path: String,
}

/// Main component implementation
struct Component;

impl Guest for Component {
    /// Initialize the build actor
    fn init(init_data: Option<Json>, _params: (String,)) -> Result<(Option<Json>,), String> {
        log("build-actor: Initializing");
        log(&format!("Initialization data: {:?}", init_data));
        Ok((init_data,))
    }
}

impl MessageServerClient for Component {
    /// Handle send messages
    fn handle_send(state: Option<Json>, _params: (Json,)) -> Result<(Option<Json>,), String> {
        // Return state unchanged
        Ok((state,))
    }

    /// Handle request messages
    fn handle_request(
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
                let mut state = BuildState {
                    store_id,
                    fs_hash,
                    status: BuildStatus::NotStarted,
                    build_output: None,
                };

                // Serialize and return state
                match serde_json::to_vec(&state) {
                    Ok(state_bytes) => {
                        log("build-actor: Initialized successfully");

                        // Start the build process
                        let res = start_build(&mut state);

                        Ok((Some(state_bytes), (serde_json::to_vec(&res).unwrap(),)))
                    }
                    Err(e) => Err(format!("Failed to serialize state: {}", e)),
                }
            }
            Err(e) => Err(format!("Failed to parse initialization data: {}", e)),
        }
    }

    fn handle_channel_open(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (bindings::exports::ntwk::theater::message_server_client::Json,),
    ) -> Result<
        (
            Option<bindings::exports::ntwk::theater::message_server_client::Json>,
            (bindings::exports::ntwk::theater::message_server_client::ChannelAccept,),
        ),
        String,
    > {
        Ok((
            state,
            (
                bindings::exports::ntwk::theater::message_server_client::ChannelAccept {
                    accepted: true,
                    message: None,
                },
            ),
        ))
    }

    fn handle_channel_close(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (String,),
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>,), String>
    {
        Ok((state,))
    }

    fn handle_channel_message(
        state: Option<bindings::exports::ntwk::theater::message_server_client::Json>,
        params: (
            String,
            bindings::exports::ntwk::theater::message_server_client::Json,
        ),
    ) -> Result<(Option<bindings::exports::ntwk::theater::message_server_client::Json>,), String>
    {
        log("runtime-content-fs: Received channel message");
        Ok((state,))
    }
}

/// Function to start the build process
fn start_build(state: &mut BuildState) -> BuildOutput {
    log("Starting build process");

    // First update state to reflect that we're starting
    let mut updated_state = state.clone();
    updated_state.status = BuildStatus::Extracting;

    match serde_json::to_vec(&updated_state) {
        Ok(state_bytes) => {
            // Begin extracting files from virtual filesystem
            let fs_hash = &state.fs_hash;

            // Directory listing from root
            match list_directory(fs_hash, "/", &state.store_id) {
                Ok(entries) => {
                    log(&format!("Found {} entries at root", entries.len()));

                    // Process entries recursively
                    match process_directory(fs_hash, "/", &entries, &state.store_id) {
                        Ok(_) => {
                            log("Successfully extracted all files from virtual filesystem");

                            // Update state to reflect we're now building
                            let mut building_state = updated_state.clone();
                            building_state.status = BuildStatus::Building;

                            if let Err(e) = serde_json::to_vec(&building_state) {
                                log(&format!("Failed to serialize building state: {}", e));
                            }

                            // Start the build process
                            match execute_build() {
                                Ok(build_output) => {
                                    log(&format!(
                                        "Build completed with success={}",
                                        build_output.success
                                    ));

                                    // Update state with build output
                                    let mut final_state = building_state.clone();
                                    final_state.build_output = Some(build_output.clone());
                                    final_state.status = if build_output.success {
                                        BuildStatus::Completed
                                    } else {
                                        BuildStatus::Failed
                                    };

                                    // Serialize final state
                                    if let Err(e) = serde_json::to_vec(&final_state) {
                                        log(&format!("Failed to serialize final state: {}", e));
                                    }

                                    build_output
                                }
                                Err(e) => {
                                    log(&format!("Build failed: {}", e));

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

                                    // Serialize failed state
                                    if let Err(e) = serde_json::to_vec(&failed_state) {
                                        log(&format!("Failed to serialize failed state: {}", e));
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
                            log(&format!("Failed to process directories: {}", e));
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
            log(&format!("Failed to serialize updated state: {}", e));
            BuildOutput {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                wasm_path: None,
                wasm_hash: None,
                build_logs: vec![],
                error: Some(e.to_string()),
            }
        }
    }
}

/// Function to process all entries in a directory recursively
fn process_directory(
    fs_hash: &str,
    path: &str,
    entries: &[DirectoryEntry],
    store_id: &str,
) -> Result<(), String> {
    for entry in entries {
        let entry_path = if path == "/" {
            format!("/{}", entry.name)
        } else {
            format!("{}/{}", path, entry.name)
        };

        log(&format!("Processing entry: {}", entry_path));

        if entry.entry_type == "directory" {
            // Create directory
            if let Err(e) = filesystem::create_dir(&format!(".{}", entry_path)) {
                log(&format!("Failed to create directory {}: {}", entry_path, e));
                return Err(format!("Failed to create directory {}: {}", entry_path, e));
            }

            // List directory contents
            match list_directory(fs_hash, &entry_path, store_id) {
                Ok(sub_entries) => {
                    // Process subdirectory
                    if let Err(e) = process_directory(fs_hash, &entry_path, &sub_entries, store_id)
                    {
                        return Err(e);
                    }
                }
                Err(e) => {
                    log(&format!("Failed to list directory {}: {}", entry_path, e));
                    return Err(format!("Failed to list directory {}: {}", entry_path, e));
                }
            }
        } else {
            // Read file content
            match read_file(fs_hash, &entry_path, store_id) {
                Ok(content) => {
                    // Write file to local filesystem
                    if let Err(e) = filesystem::write_file(&format!(".{}", entry_path), &content) {
                        log(&format!("Failed to write file {}: {}", entry_path, e));
                        return Err(format!("Failed to write file {}: {}", entry_path, e));
                    }
                }
                Err(e) => {
                    log(&format!("Failed to read file {}: {}", entry_path, e));
                    return Err(format!("Failed to read file {}: {}", entry_path, e));
                }
            }
        }
    }

    Ok(())
}

/// Function to list directory contents from the runtime store
fn list_directory(
    fs_hash: &str,
    path: &str,
    store_id: &str,
) -> Result<Vec<DirectoryEntry>, String> {
    log(&format!("Listing directory: {}", path));

    // Get the filesystem node from the store
    let content_ref = ContentRef {
        hash: fs_hash.to_string(),
    };

    // Get the content
    let fs_bytes = match store::get(store_id, &content_ref) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("Failed to retrieve filesystem root: {}", e)),
    };

    // Parse the filesystem node
    let root_node: FSNode = match serde_json::from_slice(&fs_bytes) {
        Ok(node) => node,
        Err(e) => return Err(format!("Failed to parse filesystem node: {}", e)),
    };

    // Handle root directory case
    if path == "/" {
        if root_node.node_type != NodeType::Directory || root_node.entries.is_none() {
            return Err("Root is not a valid directory".to_string());
        }

        let entries = root_node.entries.unwrap();
        let mut result = Vec::new();

        for (name, hash) in entries {
            // Get the child node to determine its type
            let child_node = get_node(&hash, store_id)?;

            let entry_type = match child_node.node_type {
                NodeType::File => "file",
                NodeType::Directory => "directory",
            };

            result.push(DirectoryEntry {
                name: name.clone(),
                entry_type: entry_type.to_string(),
                path: format!("/{}", name),
            });
        }

        return Ok(result);
    }

    // For non-root paths, navigate to the specified directory
    let path_components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    let mut current_node = root_node;
    let mut current_hash = fs_hash.to_string();

    // Navigate through path components
    for component in path_components {
        if current_node.node_type != NodeType::Directory || current_node.entries.is_none() {
            return Err(format!("Path component is not a directory: {}", component));
        }

        let entries = current_node.entries.unwrap();
        if let Some(hash) = entries.get(component) {
            current_hash = hash.clone();
            current_node = get_node(hash, store_id)?;
        } else {
            return Err(format!("Path component not found: {}", component));
        }
    }

    // Ensure the final node is a directory
    if current_node.node_type != NodeType::Directory || current_node.entries.is_none() {
        return Err(format!("Path is not a directory: {}", path));
    }

    // List entries
    let entries = current_node.entries.unwrap();
    let mut result = Vec::new();

    for (name, hash) in entries {
        // Get the child node to determine its type
        let child_node = get_node(&hash, store_id)?;

        let entry_type = match child_node.node_type {
            NodeType::File => "file",
            NodeType::Directory => "directory",
        };

        result.push(DirectoryEntry {
            name: name.clone(),
            entry_type: entry_type.to_string(),
            path: format!("{}/{}", path, name),
        });
    }

    Ok(result)
}

/// Get a node from the content store by its hash
fn get_node(hash: &str, store_id: &str) -> Result<FSNode, String> {
    let content_ref = ContentRef {
        hash: hash.to_string(),
    };

    let bytes = match store::get(store_id, &content_ref) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("Failed to retrieve node content: {}", e)),
    };

    match serde_json::from_slice::<FSNode>(&bytes) {
        Ok(node) => Ok(node),
        Err(e) => Err(format!("Failed to parse node: {}", e)),
    }
}

/// Function to read a file from the runtime store
fn read_file(fs_hash: &str, path: &str, store_id: &str) -> Result<String, String> {
    log(&format!("Reading file: {}", path));

    // First find the file node
    let file_content = read_file_content(fs_hash, path, store_id)?;

    // Convert bytes to string
    match String::from_utf8(file_content) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Failed to convert file content to string: {}", e)),
    }
}

/// Get the raw content of a file
fn read_file_content(fs_hash: &str, path: &str, store_id: &str) -> Result<Vec<u8>, String> {
    // Handle root path (invalid for a file)
    if path == "/" {
        return Err("Cannot read root directory as a file".to_string());
    }

    // Get the filesystem node from the store
    let content_ref = ContentRef {
        hash: fs_hash.to_string(),
    };

    // Get the content
    let fs_bytes = match store::get(store_id, &content_ref) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("Failed to retrieve filesystem root: {}", e)),
    };

    // Parse the filesystem node
    let root_node: FSNode = match serde_json::from_slice(&fs_bytes) {
        Ok(node) => node,
        Err(e) => return Err(format!("Failed to parse filesystem node: {}", e)),
    };

    // Split path into components
    let path_components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if path_components.is_empty() {
        return Err("Invalid file path".to_string());
    }

    let file_name = path_components[path_components.len() - 1];
    let dir_components = &path_components[0..path_components.len() - 1];

    // Navigate to parent directory
    let mut current_node = root_node;

    for component in dir_components {
        if current_node.node_type != NodeType::Directory || current_node.entries.is_none() {
            return Err(format!("Path component is not a directory: {}", component));
        }

        let entries = current_node.entries.unwrap();
        if let Some(hash) = entries.get(*component) {
            current_node = get_node(hash, store_id)?;
        } else {
            return Err(format!("Path component not found: {}", component));
        }
    }

    // Find the file in the directory
    if current_node.node_type != NodeType::Directory || current_node.entries.is_none() {
        return Err("Parent path is not a directory".to_string());
    }

    let entries = current_node.entries.unwrap();
    if let Some(file_hash) = entries.get(file_name) {
        // Get the file node
        let file_node = get_node(file_hash, store_id)?;

        // Ensure it's a file
        if file_node.node_type != NodeType::File || file_node.content.is_none() {
            return Err(format!("Path does not point to a file: {}", path));
        }

        Ok(file_node.content.unwrap())
    } else {
        Err(format!("File not found: {}", file_name))
    }
}

/// Function to execute the build process
fn execute_build() -> Result<BuildOutput, String> {
    log("Executing build command");

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

            // Capture stderr (not directly available from host function)
            let stderr = "Stderr not available from host function".to_string();

            // Check if target directory exists
            let target_dir_path = "target/wasm32-unknown-unknown/release";
            match filesystem::list_files(target_dir_path) {
                Ok(files) => {
                    log(&format!("Found {} files in target directory", files.len()));

                    // Find .wasm file
                    let wasm_files: Vec<String> = files
                        .iter()
                        .filter(|f| f.ends_with(".wasm"))
                        .cloned()
                        .collect();

                    if let Some(wasm_file) = wasm_files.first() {
                        let wasm_path = format!("{}/{}", target_dir_path, wasm_file);
                        log(&format!("Found WASM file at {}", wasm_path));

                        // Read WASM file to calculate hash
                        match filesystem::read_file(&wasm_path) {
                            Ok(wasm_bytes) => {
                                // Calculate hash (basic string hash for now)
                                let wasm_hash = format!("wasm-{}", wasm_bytes.len());

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

/// Function to send build results to the callback address
fn send_build_results(callback_address: &str, build_output: &BuildOutput) {
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

// Export the component
bindings::export!(Component with_types_in bindings);
