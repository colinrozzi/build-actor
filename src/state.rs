use serde::{Deserialize, Serialize};

/// State structure for the build actor
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildState {
    pub store_id: String,

    // Content reference to filesystem root in the Theater runtime store
    pub fs_hash: String,

    // Build status
    pub status: BuildStatus,

    // Build output information
    pub build_output: Option<BuildOutput>,

    // Channel ID for sending progress updates
    pub channel_id: Option<String>,

    // Store ID for the build output
    pub build_store_id: String,
}

/// Status of the build process
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BuildStatus {
    NotStarted,
    Extracting,
    Building,
    Completed,
    Failed,
}

/// Structure to hold build result information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub wasm_path: Option<String>,
    pub wasm_hash: Option<String>,
    pub build_logs: Vec<String>,
    pub error: Option<String>,
}

impl BuildState {
    pub fn new(store_id: String, fs_hash: String, build_store_id: String) -> Self {
        Self {
            store_id,
            fs_hash,
            status: BuildStatus::NotStarted,
            build_output: None,
            channel_id: None,
            build_store_id,
        }
    }

    pub fn set_status(&mut self, status: BuildStatus) -> &mut Self {
        self.status = status;
        self
    }

    pub fn set_build_output(&mut self, output: BuildOutput) -> &mut Self {
        self.build_output = Some(output.clone());
        if output.success {
            self.status = BuildStatus::Completed;
        } else {
            self.status = BuildStatus::Failed;
        }
        self
    }

    pub fn failed_output(error: String) -> BuildOutput {
        BuildOutput {
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            wasm_path: None,
            wasm_hash: None,
            build_logs: vec![],
            error: Some(error),
        }
    }
}
