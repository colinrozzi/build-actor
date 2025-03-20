# Build Actor

An actor that builds other actors from source code stored in the Theater runtime's content store.

## Overview

The build-actor is responsible for taking a reference to content in the Theater runtime's store (`fs_hash`), extracting the source code from it, and then building it into a WebAssembly component. It provides real-time progress updates during the build process through a channel-based communication mechanism.

## Communication Protocol

The build-actor uses a channel-based protocol that provides real-time updates during the build process:

1. A client opens a channel to the build-actor
2. The client sends a message to start the build, providing the required parameters
3. The build-actor sends progress updates through the channel as the build proceeds
4. Once complete, the build-actor sends a final completion message with the results

### Starting a Build

To start a build, send a message with the following format:

```json
{
  "command": "start_build",
  "fs_hash": "<content-hash>",
  "store_id": "<store-id>"
}
```

Parameters:
- `fs_hash`: A reference (hash) to the root filesystem node in the Theater runtime's content store
- `store_id`: The ID of the content store to use

### Build Progress Updates

The build-actor sends the following types of messages during the build process:

- Progress updates with current status and completion percentage
- Log messages with different severity levels
- File extraction notifications
- Command execution status
- Command output (stdout/stderr)
- Build completion notification

## Operation

1. The actor retrieves the source code from the content store using the provided `fs_hash`
2. It extracts the files into a local filesystem structure
3. It runs the build command (`cargo component build --target wasm32-unknown-unknown --release`)
4. It captures the build output and reports the results

## Build Status

The build process goes through several states:
- `NotStarted`: Initial state
- `Extracting`: Files are being extracted from the content store
- `Building`: Build command is executing
- `Completed`: Build finished successfully
- `Failed`: Build encountered an error

## Message Types

### Progress Update
```json
{
  "Progress": {
    "status": "Building",
    "description": "Running cargo build command",
    "percent_complete": 50.0
  }
}
```

### Log Message
```json
{
  "Log": {
    "level": "Info",
    "message": "Build command executed successfully",
    "timestamp": 1679332800
  }
}
```

### Build Complete
```json
{
  "BuildComplete": {
    "success": true,
    "wasm_path": "target/wasm32-unknown-unknown/release/my-actor.wasm",
    "wasm_hash": "wasm-12345",
    "error": null
  }
}
```