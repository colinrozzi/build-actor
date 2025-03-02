# Build Actor

An actor that builds other actors from virtual filesystem references.

## Overview

The Build Actor takes a virtual filesystem hash that references an actor project, writes that filesystem to a new temporary directory, builds the actor using nix flakes, and returns the build results to a specified callback address.

## Usage

Initialize the actor with:

```json
{
  "fs_hash": "content-fs-actor-id",
  "callback_address": "actor-address-to-receive-results"
}
```

The callback address will receive a message with the following format:

```json
{
  "action": "build_result",
  "success": true|false,
  "wasm_hash": "hash-of-compiled-wasm-if-successful",
  "logs": ["log entry 1", "log entry 2", ...],
  "error": "error message if failed",
  "stdout": "standard output from build",
  "stderr": "standard error from build"
}
```

## Features

- Takes a virtual filesystem reference and builds the actor in a temporary directory
- Uses nix flakes to ensure consistent build environment
- Returns build results to a specified callback address 
- Handles build failures gracefully
- Returns WASM file hash for successful builds

## Requirements

- Nix must be installed on the host system
- Host must have Internet access for nix to download dependencies
- Virtual filesystem must contain a valid actor project with Cargo.toml
