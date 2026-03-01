//! LSP command handler
//!
//! Starts the Language Server Protocol server for IDE integration.

use crate::utils::error::Result;

#[cfg(feature = "tower-lsp")]
use crate::lsp::server::run_lsp_server;

/// Run the LSP server
#[cfg(feature = "tower-lsp")]
pub fn run() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { run_lsp_server().await })
}
