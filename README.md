# Runtime Content Filesystem Actor

A Theater actor that provides a content-addressable virtual filesystem with Git-like versioning capabilities, integrated directly with the Theater runtime store. This actor combines traditional filesystem semantics with content-addressed storage and project/branch versioning for efficient, verifiable, and flexible file operations.

## Key Improvements Over Original Content-FS Actor

- **Direct Runtime Store Integration**: Uses the Theater runtime's built-in content-addressable store instead of relying on a separate storage actor
- **Label-Based Metadata**: Leverages the store's labeling capabilities for efficient metadata tracking
- **Simplified Architecture**: Removes the messaging layer between the content-fs actor and storage
- **Improved Performance**: Reduces latency by eliminating inter-actor communication
- **Reliability**: Eliminates potential failures in the communication between actors

## Core Features

### Content Addressing
- All file content is stored by hash (SHA1)
- Automatic deduplication of identical content
- Content integrity verification
- Efficient storage utilization

### Versioning Capabilities
- Project-based organization
- Multiple branches per project
- Commit history tracking
- Support for experimental branches
- Ability to compare and merge versions

### Filesystem Operations
- File creation and modification
- Directory management
- Path-based access
- Permission handling
- Metadata tracking

## Architecture

### State Structure
```rust
struct State {
    projects: HashMap<String, ProjectInfo>,  // Project information
    active_context: Context,                 // Current project/branch context
}

struct Context {
    project: String,
    branch: String,
    working_tree: HashMap<String, String>,   // Path to hash mapping
}

struct ProjectInfo {
    name: String,
    description: String,
    branches: HashMap<String, BranchInfo>,
    default_branch: String,
    created_at: String,
    modified_at: String,
}

struct BranchInfo {
    name: String,
    head: Option<String>,  // Commit hash
    last_commit_time: Option<String>,
}

struct Commit {
    parent: Option<String>,
    root_hash: String,
    message: String,
    timestamp: String,
    author: String,
    changes: Vec<ChangeInfo>,
}

struct FSNode {
    entries: HashMap<String, String>,  // For directories: name -> hash
    content: Vec<u8>,                  // For files: content
    node_type: NodeType,
}
```

### Storage Architecture

#### Runtime Store Integration

The actor communicates directly with the Theater runtime store, which provides:

1. **Content Storage**: The store maintains a content-addressable database using SHA1 hashes
2. **Label Management**: The store supports attaching labels to content references, which we use for:
   - Project metadata
   - Branch information
   - Commit history
   - Working tree state

#### Label Structure

We use a hierarchical labeling scheme:

- `content-fs/projects`: List of all projects
- `content-fs/project/{name}/info`: Project metadata
- `content-fs/project/{name}/branches`: List of branches
- `content-fs/project/{name}/branch/{branch-name}/info`: Branch metadata
- `content-fs/project/{name}/branch/{branch-name}/head`: Current commit reference
- `content-fs/project/{name}/branch/{branch-name}/working-tree`: Working tree state
- `content-fs/project/{name}/commits/{commit-hash}`: Commit data

### State Storage

To address the limitations of labels in the runtime store (which can only point to a single content hash), the actor follows these storage patterns:

1. **Collections as Content**: When storing collections (lists of projects, branches, etc.), the collection is serialized to JSON, stored as content, and the content hash is saved at the label.
2. **Immutable Updates**: When updating a collection, the entire collection is read, modified in memory, and then a new version is stored in its entirety.
3. **Working Trees**: Each project/branch's working tree is stored as a serialized map from paths to content hashes.

## Message Interface

The actor maintains compatibility with the original content-fs actor by implementing the same message-based interface. This means that existing clients can interact with this actor using the same JSON-based request structure:

```json
{
  "action": "read-file",
  "project": "my-project",
  "branch": "main",
  "params": {
    "path": "src/main.rs"
  }
}
```

All the original actions are supported:

### File Operations
- `read-file`: Read file content
- `write-file`: Write file content
- `list-directory`: List directory content
- `create-directory`: Create a new directory
- `delete`: Delete a file or directory

### Version Control Operations
- `commit`: Create a commit from the current working tree
- `create-branch`: Create a new branch
- `list-branches`: List all branches in a project
- `get-history`: Get commit history
- `diff`: Compare two branches

### Project Management
- `create-project`: Create a new project
- `list-projects`: List all projects
- `get-project-info`: Get information about a project

### Advanced Operations
- `search`: Search for text in files

## Implementation Notes

### 1. Storage Strategy
Each piece of content (files, directories, working trees) is stored only once in the content store, identified by its hash. Only the references (labels, directory entries) are updated when changes are made.

### 2. Content Organization
- **Files**: Stored directly as content in the store
- **Directories**: Stored as a serialized mapping from names to content hashes
- **Working Trees**: Stored as a serialized mapping from paths to content hashes
- **Metadata**: Stored as serialized JSON objects

### 3. Transaction Safety
While the store operations are not transactional, the actor maintains consistency by:
- First storing all new content
- Only then updating references
- Using atomic updates to labels

## Usage Examples

### Creating a New Project and Adding Files

```json
// Create a new project
{
  "action": "create-project",
  "params": {
    "name": "my-project",
    "description": "A sample project"
  }
}

// Create a directory
{
  "action": "create-directory",
  "project": "my-project",
  "branch": "main",
  "params": {
    "path": "/src"
  }
}

// Write a file
{
  "action": "write-file",
  "project": "my-project",
  "branch": "main",
  "params": {
    "path": "/src/main.rs",
    "content": "fn main() {\n    println!(\"Hello, world!\");\n}",
    "commit": true,
    "commit_message": "Initial commit"
  }
}
```

### Working with Branches

```json
// Create a new branch
{
  "action": "create-branch",
  "project": "my-project",
  "params": {
    "name": "feature-branch",
    "from_branch": "main"
  }
}

// Make changes on the new branch
{
  "action": "write-file",
  "project": "my-project",
  "branch": "feature-branch",
  "params": {
    "path": "/src/main.rs",
    "content": "fn main() {\n    println!(\"Hello, world from the feature branch!\");\n}",
    "commit": true,
    "commit_message": "Update greeting"
  }
}

// Compare changes with main branch
{
  "action": "diff",
  "project": "my-project",
  "params": {
    "source_branch": "main",
    "target_branch": "feature-branch",
    "path": "/src"
  }
}
```

## Future Extensions

1. **Merging capabilities** - Intelligent merging of branches
2. **Conflict resolution** - Tools for resolving merge conflicts
3. **Hooks and triggers** - Execute actions on file changes
4. **Access control** - Fine-grained permissions system
5. **Remote synchronization** - Sync between runtime instances
6. **File diffing algorithms** - Smart difference detection
7. **Large file support** - Chunking for efficient large file handling
8. **Compression** - Content-aware compression strategies
