//! LSP command handler
//!
//! Starts the Language Server Protocol server for IDE integration.

use crate::utils::error::Result;

#[cfg(feature = "tower-lsp-server")]
use crate::lsp::server::{run_lsp_server, run_lsp_server_tcp};

/// Run the LSP server
#[cfg(feature = "tower-lsp-server")]
pub fn run(port: Option<u16>) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        match port {
            Some(p) => run_lsp_server_tcp(p).await,
            None => run_lsp_server().await,
        }
    })
}
