use serde::{Deserialize, Serialize};
use crate::state::BuildStatus;

/// Build messages for communicating progress updates over a channel
#[derive(Debug, Serialize, Deserialize)]
pub enum BuildMessage {
    /// Progress update with overall status
    Progress {
        status: BuildStatus,
        description: String,
        percent_complete: Option<f32>,
    },
    
    /// Log message
    Log {
        level: LogLevel,
        message: String,
        timestamp: u64,
    },
    
    /// File extraction notification
    FileExtracted {
        path: String,
        size: usize,
    },
    
    /// Command execution started
    CommandStarted {
        command: String,
        args: Vec<String>,
    },
    
    /// Command output chunks
    CommandOutput {
        stdout: String,
        stderr: String,
    },
    
    /// Build complete notification
    BuildComplete {
        success: bool,
        wasm_path: Option<String>,
        wasm_hash: Option<String>,
        error: Option<String>,
    }
}

/// Log levels for build messages
#[derive(Debug, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}
