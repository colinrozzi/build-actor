use crate::bindings::ntwk::theater::message_server_host::send_on_channel;
use crate::bindings::ntwk::theater::runtime::log;
use crate::builder::output::BuildResultSender;
use crate::filesystem::ContentStore;
use crate::messaging::messages::{BuildMessage, LogLevel};
use crate::state::{BuildOutput, BuildState, BuildStatus};
use std::time::{SystemTime, UNIX_EPOCH};

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
    /// Send a message update through the channel if one is configured
    fn send_update(&self, message: BuildMessage) -> Result<(), String> {
        if let Some(channel_id) = &self.state.channel_id {
            let message_json = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
            send_on_channel(channel_id, &message_json).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Log a message both to the system log and through the channel
    fn log_message(&self, level: LogLevel, message: &str) {
        // Log to system
        log(message);

        // Send through channel
        let _ = self.send_update(BuildMessage::Log {
            level,
            message: message.to_string(),
        });
    }

    pub fn start(&mut self) -> BuildOutput {
        self.log_message(LogLevel::Info, "Starting build process");

        // Send initial progress update
        let _ = self.send_update(BuildMessage::Progress {
            status: BuildStatus::NotStarted,
            description: "Starting build process".to_string(),
            percent_complete: Some(0.0),
        });

        // Update state to reflect that we're starting extraction
        self.state.set_status(BuildStatus::Extracting);

        // Send extraction progress update
        let _ = self.send_update(BuildMessage::Progress {
            status: BuildStatus::Extracting,
            description: "Extracting files from content store".to_string(),
            percent_complete: Some(10.0),
        });

        // Begin extracting files from virtual filesystem
        let fs_hash = &self.state.fs_hash;

        // Extract files from virtual filesystem
        match self.content_store.extract_filesystem(fs_hash) {
            Ok(_) => {
                self.log_message(
                    LogLevel::Info,
                    "Successfully extracted all files from virtual filesystem",
                );

                // Send update that extraction is complete
                let _ = self.send_update(BuildMessage::Progress {
                    status: BuildStatus::Building,
                    description: "Files extracted, starting build".to_string(),
                    percent_complete: Some(30.0),
                });

                // Update state to reflect we're now building
                self.state.set_status(BuildStatus::Building);

                // Send build starting update
                let _ = self.send_update(BuildMessage::CommandStarted {
                    command: "nix".to_string(),
                    args: vec![
                        "develop".to_string(),
                        "--command".to_string(),
                        "bash".to_string(),
                        "-c".to_string(),
                        "cargo component build --target wasm32-unknown-unknown --release"
                            .to_string(),
                    ],
                });

                // Start the build process
                match self.execute_build() {
                    Ok(build_output) => {
                        self.log_message(
                            LogLevel::Info,
                            &format!("Build completed with success={}", build_output.success),
                        );

                        // Send completion update
                        let _ = self.send_update(BuildMessage::BuildComplete {
                            success: build_output.success,
                            wasm_path: build_output.wasm_path.clone(),
                            wasm_hash: build_output.wasm_hash.clone(),
                            error: build_output.error.clone(),
                        });

                        // Update state with build output
                        self.state.set_build_output(build_output.clone());

                        build_output
                    }
                    Err(e) => {
                        self.log_message(LogLevel::Error, &format!("Build failed: {}", e));

                        // Send failure update
                        let _ = self.send_update(BuildMessage::BuildComplete {
                            success: false,
                            wasm_path: None,
                            wasm_hash: None,
                            error: Some(e.clone()),
                        });

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
                self.log_message(
                    LogLevel::Error,
                    &format!("Failed to extract filesystem: {}", e),
                );

                // Send failure update
                let _ = self.send_update(BuildMessage::BuildComplete {
                    success: false,
                    wasm_path: None,
                    wasm_hash: None,
                    error: Some(format!("Failed to extract filesystem: {}", e)),
                });

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
        self.log_message(LogLevel::Info, "Executing build command");

        // Send progress update
        let _ = self.send_update(BuildMessage::Progress {
            status: BuildStatus::Building,
            description: "Running cargo build command".to_string(),
            percent_complete: Some(50.0),
        });

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
                self.log_message(
                    LogLevel::Info,
                    &format!("Build command executed, stdout: {}", stdout),
                );

                // Send command output
                let _ = self.send_update(BuildMessage::CommandOutput {
                    stdout: stdout.clone(),
                    stderr: "Stderr not available from host function".to_string(),
                });

                // Send progress update
                let _ = self.send_update(BuildMessage::Progress {
                    status: BuildStatus::Building,
                    description: "Build command completed, checking results".to_string(),
                    percent_complete: Some(80.0),
                });

                // Capture stderr (not directly available from host function)
                let stderr = "Stderr not available from host function".to_string();

                // Check if target directory exists
                let target_dir_path = "target/wasm32-unknown-unknown/release";
                match crate::bindings::ntwk::theater::filesystem::list_files(target_dir_path) {
                    Ok(files) => {
                        self.log_message(
                            LogLevel::Info,
                            &format!("Found {} files in target directory", files.len()),
                        );

                        // Find .wasm file
                        let wasm_files: Vec<String> = files
                            .iter()
                            .filter(|f| f.ends_with(".wasm"))
                            .cloned()
                            .collect();

                        if let Some(wasm_file) = wasm_files.first() {
                            let wasm_path = format!("{}/{}", target_dir_path, wasm_file);
                            self.log_message(
                                LogLevel::Info,
                                &format!("Found WASM file at {}", wasm_path),
                            );

                            // Send progress update
                            let _ = self.send_update(BuildMessage::Progress {
                                status: BuildStatus::Building,
                                description: "Found WASM file, calculating hash".to_string(),
                                percent_complete: Some(90.0),
                            });

                            // Read WASM file to calculate hash
                            match crate::bindings::ntwk::theater::filesystem::read_file(&wasm_path)
                            {
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
                                    self.log_message(
                                        LogLevel::Error,
                                        &format!("Failed to read WASM file: {}", e),
                                    );

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
                            self.log_message(LogLevel::Error, "No WASM file found");

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
                        self.log_message(
                            LogLevel::Error,
                            &format!("Failed to list target directory: {}", e),
                        );

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
                self.log_message(
                    LogLevel::Error,
                    &format!("Failed to execute build command: {}", e),
                );

                // Send command failure
                let _ = self.send_update(BuildMessage::CommandOutput {
                    stdout: String::new(),
                    stderr: format!("Failed to execute build command: {}", e),
                });

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
