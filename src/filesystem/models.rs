use crate::bindings::ntwk::theater::store::{self, ContentRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of filesystem nodes
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum NodeType {
    File,
    Directory,
}

/// Represents a filesystem node (file or directory)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FSNode {
    /// For directories: mapping of child names to hash
    pub entries: Option<HashMap<String, String>>,
    /// For files: content data
    pub content: Option<Vec<u8>>,
    /// Type of node
    pub node_type: NodeType,
}

/// Structure representing a directory entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub path: String,
}

/// Wrapper around the Theater runtime's content store
pub struct ContentStore {
    pub store_id: String,
}

impl ContentStore {
    pub fn new(store_id: String) -> Self {
        Self { store_id }
    }

    /// Get a node from the content store by its hash
    pub fn get_node(&self, hash: &str) -> Result<FSNode, String> {
        let content_ref = ContentRef {
            hash: hash.to_string(),
        };

        let bytes = match store::get(&self.store_id, &content_ref) {
            Ok(bytes) => bytes,
            Err(e) => return Err(format!("Failed to retrieve node content: {}", e)),
        };

        match serde_json::from_slice::<FSNode>(&bytes) {
            Ok(node) => Ok(node),
            Err(e) => Err(format!("Failed to parse node: {}", e)),
        }
    }

    /// Get a reference to the store ID
    pub fn store_id(&self) -> &str {
        &self.store_id
    }
}
