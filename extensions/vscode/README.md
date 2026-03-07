# CodeInput - CODEOWNERS Integration for VS Code

A Language Server Protocol (LSP) based extension that provides CODEOWNERS integration for Visual Studio Code. Automatically displays ownership information and tags for files in your workspace.

## Features

- **Status Bar Integration**: Shows ownership status at a glance with lock/alert icons
- **File Ownership Info**: View owners and tags for any file via command palette
- **Multi-format Support**: Works with GitHub, GitLab, and Bitbucket CODEOWNERS file locations
- **Tag Support**: Extended CODEOWNERS syntax with tag support (e.g., `*.rs @team #backend #critical`)
- **Automatic Binary Management**: Downloads the required LSP server binary automatically

## Requirements

The extension requires the `ci-lsp` binary. It will attempt to download it automatically, or you can specify a custom path in settings.

## Installation

1. Install the extension from the VS Code Marketplace
2. Open a workspace containing a CODEOWNERS file
3. The extension will activate automatically

## Configuration

| Setting                     | Description                             | Default             |
| --------------------------- | --------------------------------------- | ------------------- |
| `codeinput.binaryPath`      | Path to the codeinput LSP server binary | `ci-lsp`            |
| `codeinput.cacheFile`       | Name of the cache file                  | `.codeowners.cache` |
| `codeinput.showInStatusBar` | Show CODEOWNERS info in status bar      | `true`              |
| `codeinput.showDiagnostics` | Show diagnostics for unowned files      | `true`              |
| `codeinput.lspTransport`    | Transport method for LSP (stdio/tcp)    | `stdio`             |
| `codeinput.lspPort`         | TCP port for LSP communication          | `8123`              |

## Commands

| Command                                            | Description                                 |
| -------------------------------------------------- | ------------------------------------------- |
| `CodeInput: Show CODEOWNERS Info for Current File` | Display owners and tags for the active file |
| `CodeInput: Refresh CODEOWNERS Cache`              | Force refresh the ownership cache           |

## CODEOWNERS File Locations

The extension monitors these locations for CODEOWNERS files:

- `CODEOWNERS` (root)
- `.github/CODEOWNERS`
- `.gitlab/CODEOWNERS`
- `docs/CODEOWNERS`

## Extended Syntax

CodeInput supports an extended CODEOWNERS syntax with tags:

```
# Standard GitHub syntax
*.rs @rust-team

# Extended syntax with tags
src/api/** @backend-team #api #critical
*.md @docs-team #documentation
```

## License

MIT
