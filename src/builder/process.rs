use crate::bindings::ntwk::theater::runtime::log;
use crate::builder::output::BuildResultSender;
use crate::filesystem::ContentStore;
use crate::state::{BuildOutput, BuildState, BuildStatus};

/// Structure to manage the build process
pub struct BuildProcess {
    state: BuildState,
    content_store: ContentStore,
}

impl BuildProcess {
    pub fn new(state: BuildState) -> Self {
        let store_id = state.store_id.clone();
        Self {
            state,
            content_store: ContentStore::new(store_id),
        }
    }

    /// Function to start the build process
    pub fn start(&mut self) -> BuildOutput {
        log("Starting build process");

        // Update state to reflect that we're starting extraction
        self.state.set_status(BuildStatus::Extracting);

        // Begin extracting files from virtual filesystem
        let fs_hash = &self.state.fs_hash;

        // Extract files from virtual filesystem
        match self.content_store.extract_filesystem(fs_hash) {
            Ok(_) => {
                log("Successfully extracted all files from virtual filesystem");

                // Update state to reflect we're now building
                self.state.set_status(BuildStatus::Building);

                // Start the build process
                match self.execute_build() {
                    Ok(build_output) => {
                        log(&format!(
                            "Build completed with success={}",
                            build_output.success
                        ));

                        // Update state with build output
                        self.state.set_build_output(build_output.clone());

                        build_output
                    }
                    Err(e) => {
                        log(&format!("Build failed: {}", e));

                        // Create failure output
                        let failed_output = BuildOutput {
                            success: false,
                            stdout: String::new(),
                            stderr: String::new(),
                            wasm_path: None,
                            wasm_hash: None,
                            build_logs: vec![],
                            error: Some(e.clone()),
                        };

                        // Update state
                        self.state.set_build_output(failed_output.clone());

                        failed_output
                    }
                }
            }
            Err(e) => {
                log(&format!("Failed to extract filesystem: {}", e));
                
                // Create failure output
                let failed_output = BuildOutput {
                    success: false,
                    stdout: String::new(),
                    stderr: String::new(),
                    wasm_path: None,
                    wasm_hash: None,
                    build_logs: vec![],
                    error: Some(e),
                };

                // Update state
                self.state.set_build_output(failed_output.clone());

                failed_output
            }
        }
    }

    /// Get a reference to the current build state
    pub fn state(&self) -> &BuildState {
        &self.state
    }

    /// Function to execute the build process
    fn execute_build(&self) -> Result<BuildOutput, String> {
        log("Executing build command");

        // Execute the nix build command
        match crate::bindings::ntwk::theater::filesystem::execute_command(
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
                match crate::bindings::ntwk::theater::filesystem::list_files(target_dir_path) {
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
                            match crate::bindings::ntwk::theater::filesystem::read_file(&wasm_path) {
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
}
