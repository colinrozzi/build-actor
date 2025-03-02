mod bindings;
mod handlers;
/// Content Filesystem Actor (Runtime Store Integration)
///
/// A content-addressable virtual filesystem with Git-like versioning capabilities.
/// Enhanced to use the Theater runtime store directly.
mod models;
mod storage;
mod utils;

use bindings::ntwk::theater::runtime::log;
use bindings::ntwk::theater::types::Json;
use models::init::InitData;
use models::{Request, State};
use storage::StorageInterface;
use utils::error_response;

use models::{BranchInfo, ProjectInfo};
use std::collections::HashMap;

use bindings::exports::ntwk::theater::actor::Guest;
use bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClient;

/// Main component implementation for the runtime-content-fs actor
struct Component;

impl Guest for Component {
    /// Initialize the component
    fn init(init_data: Option<Json>, _params: (String,)) -> Result<(Option<Json>,), String> {
        log("runtime-content-fs: Initializing");

        // Parse initialization data if provided
        let state = if let Some(data) = init_data {
            match serde_json::from_slice::<InitData>(&data) {
                Ok(init) => {
                    log("runtime-content-fs: Found initialization data");
                    // Create a new state
                    let mut state = State::default();

                    // Check if we should create a new filesystem
                    let create_new = init.create_new.unwrap_or(false);

                    if create_new {
                        log("runtime-content-fs: Creating new filesystem");

                        // Create default project with customized name if provided
                        let project_name = init
                            .default_project_name
                            .unwrap_or_else(|| "default".to_string());
                        let project_description = init
                            .default_project_description
                            .unwrap_or_else(|| "Default project".to_string());

                        // Create main branch
                        let main_branch = BranchInfo {
                            name: "main".to_string(),
                            head: None,
                        };

                        let mut branches = HashMap::new();
                        branches.insert("main".to_string(), main_branch);

                        // Create the project
                        let project = ProjectInfo {
                            name: project_name.clone(),
                            description: project_description,
                            branches,
                            default_branch: "main".to_string(),
                        };

                        // Add project to state
                        state.projects.insert(project_name.clone(), project);

                        // Set as active project and branch
                        state.active_context.project = project_name;
                        state.active_context.branch = "main".to_string();
                        state.active_context.working_tree = HashMap::new();

                        // Initialize storage
                        let storage = StorageInterface::new();

                        // Store initial project list
                        match storage.store_project_list(&[state.active_context.project.clone()]) {
                            Ok(_) => log("runtime-content-fs: Stored project list"),
                            Err(e) => log(&format!(
                                "runtime-content-fs: Failed to store project list: {}",
                                e
                            )),
                        }

                        // Store project info
                        match storage.store_project_info(
                            &state.active_context.project,
                            &serde_json::to_value(
                                &state.projects.get(&state.active_context.project).unwrap(),
                            )
                            .unwrap(),
                        ) {
                            Ok(_) => log("runtime-content-fs: Stored project info"),
                            Err(e) => log(&format!(
                                "runtime-content-fs: Failed to store project info: {}",
                                e
                            )),
                        }

                        // Store branch list
                        match storage
                            .store_branch_list(&state.active_context.project, &["main".to_string()])
                        {
                            Ok(_) => log("runtime-content-fs: Stored branch list"),
                            Err(e) => log(&format!(
                                "runtime-content-fs: Failed to store branch list: {}",
                                e
                            )),
                        }

                        // Store branch info
                        match storage.store_branch_info(
                            &state.active_context.project,
                            "main",
                            &serde_json::to_value(
                                &state
                                    .projects
                                    .get(&state.active_context.project)
                                    .unwrap()
                                    .branches
                                    .get("main")
                                    .unwrap(),
                            )
                            .unwrap(),
                        ) {
                            Ok(_) => log("runtime-content-fs: Stored branch info"),
                            Err(e) => log(&format!(
                                "runtime-content-fs: Failed to store branch info: {}",
                                e
                            )),
                        }

                        // Create root directory
                        use crate::models::FSNode;
                        use crate::models::NodeType;

                        let root_dir = FSNode {
                            entries: Some(HashMap::new()),
                            content: None,
                            node_type: NodeType::Directory,
                        };

                        let root_hash = match storage.store_node(&root_dir) {
                            Ok(hash) => hash,
                            Err(e) => {
                                log(&format!(
                                    "runtime-content-fs: Failed to store root directory: {}",
                                    e
                                ));
                                return Err(format!("Failed to store root directory: {}", e));
                            }
                        };

                        // Create working tree with root directory
                        let mut working_tree = HashMap::new();
                        working_tree.insert("/".to_string(), root_hash.clone());

                        // Store working tree
                        match storage.store_working_tree(
                            &state.active_context.project,
                            "main",
                            &working_tree,
                        ) {
                            Ok(_) => log("runtime-content-fs: Stored empty working tree"),
                            Err(e) => log(&format!(
                                "runtime-content-fs: Failed to store working tree: {}",
                                e
                            )),
                        }

                        log("runtime-content-fs: New filesystem created successfully");
                    } else {
                        // Using existing filesystem
                        // Set active project if provided and exists
                        if let Some(project) = init.root_project {
                            log(&format!(
                                "runtime-content-fs: Setting active project: {}",
                                project
                            ));

                            // Get project list from storage
                            let storage = StorageInterface::new();
                            match storage.get_project_list() {
                                Ok(projects) => {
                                    if projects.contains(&project) {
                                        state.active_context.project = project;
                                    } else {
                                        log(&format!(
                                            "runtime-content-fs: Project not found: {}",
                                            project
                                        ));
                                    }
                                }
                                Err(e) => log(&format!(
                                    "runtime-content-fs: Failed to get project list: {}",
                                    e
                                )),
                            }
                        }

                        // Set active branch if provided and exists
                        if let Some(branch) = init.default_branch {
                            log(&format!(
                                "runtime-content-fs: Setting active branch: {}",
                                branch
                            ));

                            let storage = StorageInterface::new();
                            match storage.get_branch_list(&state.active_context.project) {
                                Ok(branches) => {
                                    if branches.contains(&branch) {
                                        state.active_context.branch = branch;
                                    } else {
                                        log(&format!(
                                            "runtime-content-fs: Branch not found: {}",
                                            branch
                                        ));
                                    }
                                }
                                Err(e) => log(&format!(
                                    "runtime-content-fs: Failed to get branch list: {}",
                                    e
                                )),
                            }
                        }
                    }

                    state
                }
                Err(e) => {
                    log(&format!(
                        "runtime-content-fs: Failed to parse init data: {}",
                        e
                    ));
                    State::default()
                }
            }
        } else {
            log("runtime-content-fs: No initialization data provided, using defaults");
            State::default()
        };

        // Serialize the initial state
        match serde_json::to_vec(&state) {
            Ok(state_bytes) => {
                log("runtime-content-fs: Initialized successfully");
                Ok((Some(state_bytes),))
            }
            Err(e) => {
                log(&format!(
                    "runtime-content-fs: Failed to serialize initial state: {}",
                    e
                ));
                Err(format!("Failed to serialize initial state: {}", e))
            }
        }
    }
}

impl MessageServerClient for Component {
    /// Handle send messages (not used in this actor)
    fn handle_send(state: Option<Json>, _params: (Json,)) -> Result<(Option<Json>,), String> {
        // Just return the state unchanged
        Ok((state,))
    }

    /// Handle request messages
    fn handle_request(
        state_bytes: Option<Json>,
        params: (Json,),
    ) -> Result<(Option<Json>, (Json,)), String> {
        log("runtime-content-fs: Received message");

        // Deserialize state or create default
        let mut state: State = match state_bytes {
            Some(bytes) => match serde_json::from_slice(&bytes) {
                Ok(s) => s,
                Err(e) => {
                    log(&format!("Failed to deserialize state: {}", e));
                    State::default()
                }
            },
            None => State::default(),
        };

        // Process the request
        let request_bytes = params.0;
        let request_str = match std::str::from_utf8(&request_bytes) {
            Ok(s) => s,
            Err(e) => {
                log(&format!("Invalid UTF-8 in request: {}", e));
                return Ok((
                    Some(serde_json::to_vec(&state).unwrap()),
                    (error_response("Invalid UTF-8 in request").into_bytes(),),
                ));
            }
        };

        // Process request and get response
        let response = match process_request(&mut state, request_str) {
            Ok(resp) => resp,
            Err(e) => error_response(&format!("Error processing request: {}", e)),
        };

        // Return updated state and response
        Ok((
            Some(serde_json::to_vec(&state).unwrap()),
            (response.into_bytes(),),
        ))
    }
}

/// Process a request and return a response
fn process_request(state: &mut State, request_json: &str) -> Result<String, String> {
    log(&format!("Processing request: {}", request_json));

    // Parse the request
    let request: Request = match serde_json::from_str(request_json) {
        Ok(req) => req,
        Err(e) => {
            return Ok(error_response(&format!("Invalid request JSON: {}", e)));
        }
    };

    // Set context from request
    if let Some(project) = &request.project {
        // Validate project exists using storage
        let storage = StorageInterface::new();
        let projects = match storage.get_project_list() {
            Ok(p) => p,
            Err(e) => {
                return Ok(error_response(&format!(
                    "Failed to get project list: {}",
                    e
                )))
            }
        };

        if !projects.contains(project) {
            return Ok(error_response(&format!("Project '{}' not found", project)));
        }

        state.active_context.project = project.clone();
    }

    if let Some(branch) = &request.branch {
        // Validate branch exists using storage
        let storage = StorageInterface::new();
        let branches = match storage.get_branch_list(&state.active_context.project) {
            Ok(b) => b,
            Err(e) => return Ok(error_response(&format!("Failed to get branch list: {}", e))),
        };

        if !branches.contains(branch) {
            return Ok(error_response(&format!("Branch '{}' not found", branch)));
        }

        state.active_context.branch = branch.clone();
    }

    // Dispatch to appropriate handler based on action
    let response = match request.action.as_str() {
        // File operations
        "read-file" => handle_read_file(state, &request),
        "write-file" => handle_write_file(state, &request),
        "list-directory" => handle_list_directory(state, &request),
        "create-directory" => handle_create_directory(state, &request),
        "delete" => handle_delete(state, &request),

        // Version control operations
        "commit" => handle_commit(state, &request),
        "create-branch" => handle_create_branch(state, &request),
        "list-branches" => handle_list_branches(state, &request),
        "get-history" => handle_get_history(state, &request),
        "diff" => handle_diff(state, &request),

        // Project management
        "create-project" => handle_create_project(state, &request),
        "list-projects" => handle_list_projects(state, &request),
        "get-project-info" => handle_get_project_info(state, &request),

        // Advanced operations
        "search" => handle_search(state, &request),

        // Unknown action
        _ => error_response(&format!("Unknown action: {}", request.action)),
    };

    Ok(response)
}

// We'll define handler functions that delegate to our handlers module
fn handle_read_file(state: &State, request: &Request) -> String {
    // Convert the request parameters to path
    let path = match request.params.get("path").and_then(|p| p.as_str()) {
        Some(p) => p.to_string(),
        None => return utils::error_response("Missing 'path' parameter"),
    };

    // Get the project and branch names
    let project = &state.active_context.project;
    let branch = &state.active_context.branch;

    // Call the actual read_file function
    match state.read_file(project, branch, &path) {
        Ok(data) => utils::success_response(serde_json::to_value(data).unwrap_or_default()),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_write_file(state: &mut State, request: &Request) -> String {
    match state.handle_write_file(&request.params) {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_list_directory(state: &State, request: &Request) -> String {
    match state.handle_list_directory(&request.params) {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_create_directory(state: &mut State, request: &Request) -> String {
    match state.handle_create_directory(&request.params) {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_delete(state: &mut State, request: &Request) -> String {
    match state.handle_delete(&request.params) {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_commit(state: &mut State, request: &Request) -> String {
    match state.handle_commit(&request.params) {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_create_branch(state: &mut State, request: &Request) -> String {
    match state.handle_create_branch(&request.params) {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_list_branches(state: &State, _request: &Request) -> String {
    match state.handle_list_branches() {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_get_history(state: &State, request: &Request) -> String {
    match state.handle_get_history(&request.params) {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_diff(state: &State, request: &Request) -> String {
    match state.handle_diff(&request.params) {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_create_project(state: &mut State, request: &Request) -> String {
    match state.handle_create_project(&request.params) {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_list_projects(state: &State, _request: &Request) -> String {
    match state.handle_list_projects() {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_get_project_info(state: &State, _request: &Request) -> String {
    match state.handle_get_project_info() {
        Ok(data) => utils::success_response(data),
        Err(e) => utils::error_response(&e.to_string()),
    }
}

fn handle_search(_state: &State, _request: &Request) -> String {
    // Placeholder for now - this can be implemented later
    utils::success_response(serde_json::json!({
        "matches": [],
        "total": 0,
        "query": "search-query",
    }))
}

// Export the component
bindings::export!(Component with_types_in bindings);
