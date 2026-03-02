# Changelog

All notable changes to this project will be documented in this file.

## [0.0.5] - 2025-03-02

### Added
- **LSP Server**: Language Server Protocol implementation for IDE integration
  - `textDocument/hover`: Shows owners and tags when hovering over files
  - `textDocument/codeLens`: Displays ownership annotations above files
  - `textDocument/publishDiagnostics`: Warns about unowned files
  - Multi-root workspace support
- **TCP Transport**: LSP server now supports TCP connections (`ci lsp --port <PORT>`)
- **VS Code Extension**: Companion extension for Visual Studio Code (in `vscode-extension/`)

### Changed
- Upgraded `utoipa` to 5.4.0 with schema fixes
- LSP feature is now opt-in via `--features lsp` cargo flag

### Fixed
- Safe serialization of CodeLens arguments
- Output redirected to stderr for LSP compatibility

## [0.0.4] - 2024-12-XX

### Added
- `infer-owners` command for intelligent owner inference
- ARM64 runners for release builds (Linux, Windows, macOS)

### Changed
- Dependency updates and formatting improvements

## [0.0.3] - Previous Release

Initial public release with core CODEOWNERS parsing and analysis features.
