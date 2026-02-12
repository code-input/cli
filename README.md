<h1 align="center">Code Input CLI</h1>
<div align="center">
 <strong>
   Advanced CODEOWNERS file management and analysis toolkit
 </strong>
</div>
<br/>

[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/CodeInputCorp/cli/blob/master/LICENSE)
[![Tests](https://github.com/CodeInputCorp/cli/actions/workflows/tests.yml/badge.svg)](https://github.com/CodeInputCorp/cli/actions/workflows/tests.yml)
[![Build](https://github.com/CodeInputCorp/cli/actions/workflows/build.yml/badge.svg)](https://github.com/CodeInputCorp/cli/actions/workflows/build.yml)

`codeinput` is a CLI tool for parsing, analyzing, and managing CODEOWNERS files across your repository. It provides advanced querying capabilities, ownership analysis, and tag-based file organization.

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
**Table of Contents**

- [Features](#features)
  - [CodeOwners Features](#codeowners-features)
- [Installation](#installation)
  - [Pre-built Binaries](#pre-built-binaries)
    - [Installation Instructions](#installation-instructions)
  - [From Cargo](#from-cargo)
  - [From Source](#from-source)
  - [From NPM](#from-npm)
- [Quick Start](#quick-start)
- [Commands](#commands)
  - [CodeOwners](#codeowners)
    - [Parse CODEOWNERS](#parse-codeowners)
    - [List Files](#list-files)
    - [List Owners](#list-owners)
    - [List Tags](#list-tags)
    - [Inspect Files](#inspect-files)
  - [Configuration](#configuration)
  - [Shell Completion](#shell-completion)
- [CODEOWNERS Format](#codeowners-format)
  - [1. Traditional CODEOWNERS Files](#1-traditional-codeowners-files)
  - [2. Inline Per-File Ownership](#2-inline-per-file-ownership)
- [How to Contribute](#how-to-contribute)
- [License](#license)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Features

### CodeOwners Features

- **Advanced Parsing**: Parse CODEOWNERS files recursively across directory structures
- **Ownership Analysis**: Analyze file ownership patterns and generate detailed reports
- **Tag Support**: Organize and query files using custom tags in CODEOWNERS
- **High Performance**: Efficient caching and parallel processing for large repositories
- **Flexible Filtering**: Filter files by owners, tags, or ownership status
- **Multiple Output Formats**: Support for text, JSON, and binary output formats

## Installation

<!-- RELEASE_INFO_START -->

### Pre-built Binaries

**Latest Release: `0.0.4`**

- **Linux x86_64**: [Download](https://github.com/CodeInputCorp/cli/releases/download/v0.0.4/ci-linux-x86_64)
- **Linux ARM64**: [Download](https://github.com/CodeInputCorp/cli/releases/download/v0.0.4/ci-linux-aarch64)
- **Windows x86_64**: [Download](https://github.com/CodeInputCorp/cli/releases/download/v0.0.4/ci-windows-x86_64.exe)
- **macOS Intel**: [Download](https://github.com/CodeInputCorp/cli/releases/download/v0.0.4/ci-macos-x86_64)
- **macOS Apple Silicon**: [Download](https://github.com/CodeInputCorp/cli/releases/download/v0.0.4/ci-macos-aarch64)

#### Installation Instructions

1. Download the appropriate binary for your platform
2. Rename the downloaded file to `ci` (Linux/macOS) or `ci.exe` (Windows)
3. Move the binary to your PATH: `mv ci /usr/local/bin/` (Linux/macOS)
4. Make it executable: `chmod +x /usr/local/bin/ci` (Linux/macOS)

<!-- RELEASE_INFO_END -->

### From Cargo

You can install the CLI using Cargo. Rust Toolchain required.

```bash
cargo install ci
```

### From Source

```bash
git clone https://github.com/CodeInputCorp/cli.git
cd cli
cargo build --release --bin ci
sudo cp target/release/ci /usr/local/bin/
```

### From NPM

```bash
npm install -g @codeinput/codeinput
```

## Quick Start

1. **Parse your CODEOWNERS files** to build the cache:

   ```bash
   ci codeowners parse
   ```

2. **List all files with their owners**:

   ```bash
   ci codeowners list-files
   ```

3. **Find files owned by a specific team**:

   ```bash
   ci codeowners list-files --owners @frontend-team
   ```

4. **Inspect ownership of a specific file**:
   ```bash
   ci codeowners inspect src/main.rs
   ```

## Commands

### CodeOwners

#### Parse CODEOWNERS

Build a cache of parsed CODEOWNERS files for fast querying:

```bash
ci codeowners parse [PATH] [OPTIONS]
```

**Options:**

- `--cache-file <FILE>`: Custom cache file location (default: `.codeowners.cache`)
- `--format <FORMAT>`: Cache format - `bincode` or `json` (default: `bincode`)

**Examples:**

```bash
# Parse current directory
ci codeowners parse

# Parse specific directory with JSON cache
ci codeowners parse ./my-repo --format json

# Use custom cache location
ci codeowners parse --cache-file .custom-cache
```

#### List Files

Find and list files with their owners based on filter criteria:

```bash
ci codeowners list-files [PATH] [OPTIONS]
```

**Options:**

- `--tags <LIST>`: Filter by tags (comma-separated)
- `--owners <LIST>`: Filter by owners (comma-separated)
- `--unowned`: Show only unowned files
- `--show-all`: Show all files including unowned/untagged
- `--format <FORMAT>`: Output format - `text`, `json`, or `bincode`

**Examples:**

```bash
# List all owned files
ci codeowners list-files

# Find files with specific tags
ci codeowners list-files --tags security critical

# Find files owned by multiple teams
ci codeowners list-files --owners @backend-team @devops

# Show unowned files
ci codeowners list-files --unowned

# Output as JSON
ci codeowners list-files --format json
```

#### List Owners

Display aggregated owner statistics and file associations:

```bash
ci codeowners list-owners [PATH] [OPTIONS]
```

**Options:**

- `--format <FORMAT>`: Output format - `text`, `json`, or `bincode`

**Examples:**

```bash
# Show all owners with file counts
ci codeowners list-owners

# Get owner data as JSON
ci codeowners list-owners --format json
```

#### List Tags

Analyze tag usage across CODEOWNERS files:

```bash
ci codeowners list-tags [PATH] [OPTIONS]
```

**Options:**

- `--format <FORMAT>`: Output format - `text`, `json`, or `bincode`

**Examples:**

```bash
# Show all tags with usage statistics
ci codeowners list-tags

# Export tag data as JSON
ci codeowners list-tags --format json
```

#### Inspect Files

Get detailed ownership and tag information for a specific file:

```bash
ci codeowners inspect <FILE_PATH> [OPTIONS]
```

**Options:**

- `--repo <PATH>`: Repository path (default: current directory)
- `--format <FORMAT>`: Output format - `text`, `json`, or `bincode`

**Examples:**

```bash
# Inspect a specific file
ci codeowners inspect src/main.rs

# Inspect with different repo path
ci codeowners inspect src/main.rs --repo /path/to/repo

# Get inspection data as JSON
ci codeowners inspect src/main.rs --format json
```

### Configuration

View current configuration settings:

```bash
ci config
```

### Shell Completion

Generate shell completion scripts:

```bash
# For bash
ci completion bash > /etc/bash_completion.d/codeinput

# For zsh
ci completion zsh > ~/.zsh/completions/_codeinput

# For fish
ci completion fish > ~/.config/fish/completions/codeinput.fish
```

## CODEOWNERS Format

The tool supports two approaches for defining code ownership:

### 1. Traditional CODEOWNERS Files

Standard CODEOWNERS syntax with additional tag support:

```
# Standard ownership
*.rs @rust-team
/docs/ @documentation-team

# With tags for categorization
*.security.rs @security-team #security #critical
/api/ @backend-team #api #core

# Multiple owners and tags
*.config @devops @security-team #config #infrastructure #security

# Special patterns
* @default-team #general
```

**Pattern Matching Rules:**

- `/path/to/dir/` - Matches all files and subdirectories recursively (GitHub compatible)
- `/path/to/dir/*` - Matches direct files in the directory only
- `/path/to/dir/**` - Matches all files and subdirectories recursively (explicit)
- `*.ext` - Matches files with specific extension
- `pattern` - Relative path matching

**Priority Rules:**

1. **Closest CODEOWNERS file**: Files in subdirectories take precedence over parent directories
2. **Last match wins**: Later entries in the same CODEOWNERS file override earlier ones
3. **Inline declarations**: Per-file inline ownership

### 2. Inline Per-File Ownership

For fine-grained control, declare ownership directly within individual files using the `!!!CODEOWNERS` marker:

**Rust Example:**

```rust
// !!!CODEOWNERS @security-team @alice #security #critical
use std::crypto;

fn encrypt_data() {
    // sensitive crypto code
}
```

**JavaScript/TypeScript Example:**

```javascript
// !!!CODEOWNERS @frontend-team #ui #components
export const Button = () => {
  return <button>Click me</button>;
};
```

**HTML Example:**

```html
<!-- !!!CODEOWNERS @design-team #ui #templates -->
<div class="hero-section">
  <h1>Welcome</h1>
</div>
```

**CSS Example:**

```css
/* !!!CODEOWNERS @design-team #styles #branding */
.brand-colors {
  --primary: #007bff;
  --secondary: #6c757d;
}
```

**Inline Format Rules:**

- Must appear within the first 50 lines of the file
- Can be used in any comment style (`//`, `#`, `/**/`, `<!-- -->`)
- Takes highest priority over all CODEOWNERS file patterns
- Supports same owner and tag syntax as CODEOWNERS files
- Only one inline declaration per file (first one found is used)

**Supported Owner Types:**

- **Users**: `@username`
- **Teams**: `@org/team-name`
- **Email**: `user@example.com`

**Tag Format:**

- Tags start with `#` and appear after owners
- Multiple tags are supported: `#tag1 #tag2 #tag3`
- Tags can contain letters, numbers, hyphens, and underscores
- Comments after tags are ignored (e.g., `#tag1 # this is a comment`)

## How to Contribute

We welcome contributions! Please see our [Contributing Guide](.github/CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
