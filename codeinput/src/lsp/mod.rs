//! Language Server Protocol (LSP) implementation for CODEOWNERS integration
//!
//! This module provides an LSP server that integrates with the codeinput CLI
//! to provide real-time CODEOWNERS information in editors like VS Code, Neovim, etc.

pub mod commands;
pub mod handlers;
pub mod server;
pub mod types;

pub use server::LspServer;
