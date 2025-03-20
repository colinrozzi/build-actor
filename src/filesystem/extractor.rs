use crate::bindings::ntwk::theater::filesystem;
use crate::bindings::ntwk::theater::runtime::log;
use crate::filesystem::models::{ContentStore, DirectoryEntry};

/// Functions for extracting files from the virtual filesystem to the local filesystem
impl ContentStore {
    /// Function to process all entries in a directory recursively
    pub fn process_directory(
        &self,
        fs_hash: &str,
        path: &str,
        entries: &[DirectoryEntry],
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
                match self.list_directory(fs_hash, &entry_path) {
                    Ok(sub_entries) => {
                        // Process subdirectory
                        if let Err(e) = self.process_directory(fs_hash, &entry_path, &sub_entries) {
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
                match self.read_file(fs_hash, &entry_path) {
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

    /// Extract all files from a virtual filesystem to the local filesystem
    pub fn extract_filesystem(&self, fs_hash: &str) -> Result<(), String> {
        log("Starting extraction process");

        // Directory listing from root
        match self.list_directory(fs_hash, "/") {
            Ok(entries) => {
                log(&format!("Found {} entries at root", entries.len()));

                // Process entries recursively
                match self.process_directory(fs_hash, "/", &entries) {
                    Ok(_) => {
                        log("Successfully extracted all files from virtual filesystem");
                        Ok(())
                    }
                    Err(e) => {
                        log(&format!("Failed to process directories: {}", e));
                        Err(e)
                    }
                }
            }
            Err(e) => {
                log(&format!("Failed to list root directory: {}", e));
                Err(e)
            }
        }
    }
}
