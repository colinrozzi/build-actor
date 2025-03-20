use std::fmt;

/// Custom error type for the build actor
#[derive(Debug)]
pub enum BuildActorError {
    /// Errors related to content store operations
    StoreError(String),
    
    /// Errors related to filesystem operations
    FilesystemError(String),
    
    /// Errors related to the build process
    BuildError(String),
    
    /// Errors related to serialization/deserialization
    SerializationError(String),
    
    /// Errors related to configuration/parameters
    ConfigurationError(String),
    
    /// Other general errors
    OtherError(String),
}

impl fmt::Display for BuildActorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildActorError::StoreError(msg) => write!(f, "Store error: {}", msg),
            BuildActorError::FilesystemError(msg) => write!(f, "Filesystem error: {}", msg),
            BuildActorError::BuildError(msg) => write!(f, "Build error: {}", msg),
            BuildActorError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            BuildActorError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            BuildActorError::OtherError(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for BuildActorError {}

/// Result type alias for BuildActorError
pub type Result<T> = std::result::Result<T, BuildActorError>;

/// Convert from String error to BuildActorError
impl From<String> for BuildActorError {
    fn from(error: String) -> Self {
        BuildActorError::OtherError(error)
    }
}

/// Convert from serde_json::Error to BuildActorError
impl From<serde_json::Error> for BuildActorError {
    fn from(error: serde_json::Error) -> Self {
        BuildActorError::SerializationError(error.to_string())
    }
}

/// Convert from std::string::FromUtf8Error to BuildActorError
impl From<std::string::FromUtf8Error> for BuildActorError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        BuildActorError::OtherError(error.to_string())
    }
}
