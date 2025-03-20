# Changelog

## [0.3.0] - 2025-03-20

### Added
- Converted build process to use channel-based communication
- Added real-time progress updates during build process
- Implemented structured message format for build events
- Added support for querying build status via channel
- Enhanced logging with severity levels

## [0.2.0] - 2025-03-04

### Changed
- Updated implementation to use the Theater runtime store API directly
- Changed `fs_hash` to reference content in the runtime store instead of an actor ID
- Added proper filesystem node traversal using content references
- Improved error handling for content store operations

## [0.1.0] - Initial version

### Added
- Initial implementation using actor messaging for file access
- Basic build functionality for Rust projects
- Support for callback notifications