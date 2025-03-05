# Build Actor

An actor that builds other actors from source code stored in the Theater runtime's content store.

## Overview

The build-actor is responsible for taking a reference to content in the Theater runtime's store (`fs_hash`), extracting the source code from it, and then building it into a WebAssembly component.

## Parameters

- `fs_hash`: A reference (hash) to the root filesystem node in the Theater runtime's content store
- `callback_address`: The address to send build results to when the build is complete

## Operation

1. The actor retrieves the source code from the content store using the provided `fs_hash`
2. It extracts the files into a local filesystem structure
3. It runs the build command (`cargo build --target wasm32-unknown-unknown --release`)
4. It captures the build output and sends it to the callback address

## Status

The build process goes through several states:
- `NotStarted`: Initial state
- `Extracting`: Files are being extracted from the content store
- `Building`: Build command is executing
- `Completed`: Build finished successfully
- `Failed`: Build encountered an error