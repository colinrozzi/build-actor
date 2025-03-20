use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::store::{self, ContentRef};
use crate::filesystem::models::{ContentStore, DirectoryEntry, FSNode, NodeType};

/// Functions for reading data from the virtual filesystem in the content store
impl ContentStore {
    /// Function to list directory contents from the runtime store
    pub fn list_directory(&self, fs_hash: &str, path: &str) -> Result<Vec<DirectoryEntry>, String> {
        log(&format!("Listing directory: {}", path));

        // Get the filesystem node from the store
        let content_ref = ContentRef {
            hash: fs_hash.to_string(),
        };

        // Get the content
        let fs_bytes = match store::get(&self.store_id, &content_ref) {
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
                let child_node = self.get_node(&hash)?;

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
                current_node = self.get_node(hash)?;
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
            let child_node = self.get_node(&hash)?;

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

    /// Function to read a file from the runtime store
    pub fn read_file(&self, fs_hash: &str, path: &str) -> Result<String, String> {
        log(&format!("Reading file: {}", path));

        // First find the file node
        let file_content = self.read_file_content(fs_hash, path)?;

        // Convert bytes to string
        match String::from_utf8(file_content) {
            Ok(content) => Ok(content),
            Err(e) => Err(format!("Failed to convert file content to string: {}", e)),
        }
    }

    /// Get the raw content of a file
    pub fn read_file_content(&self, fs_hash: &str, path: &str) -> Result<Vec<u8>, String> {
        // Handle root path (invalid for a file)
        if path == "/" {
            return Err("Cannot read root directory as a file".to_string());
        }

        // Get the filesystem node from the store
        let content_ref = ContentRef {
            hash: fs_hash.to_string(),
        };

        // Get the content
        let fs_bytes = match store::get(&self.store_id, &content_ref) {
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
                current_node = self.get_node(hash)?;
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
            let file_node = self.get_node(file_hash)?;

            // Ensure it's a file
            if file_node.node_type != NodeType::File || file_node.content.is_none() {
                return Err(format!("Path does not point to a file: {}", path));
            }

            Ok(file_node.content.unwrap())
        } else {
            Err(format!("File not found: {}", file_name))
        }
    }
}
