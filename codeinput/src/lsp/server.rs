use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::Client;
use url::Url;

use crate::core::cache::sync_cache;
use crate::core::types::{CodeownersCache, OwnerType};
use crate::utils::error::{Error, Result};

use super::types::FileOwnershipInfo;

/// LSP Server state
pub struct LspServer {
    pub client: Client,
    /// Map of workspace root URIs to their cached CODEOWNERS data
    pub workspaces: Arc<RwLock<HashMap<Url, WorkspaceState>>>,
}

/// State for a single workspace
#[derive(Debug)]
pub struct WorkspaceState {
    pub cache: CodeownersCache,
    pub cache_file: Option<PathBuf>,
}

impl LspServer {
    /// Create a new LSP server instance
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workspaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize a workspace by loading its CODEOWNERS cache
    pub async fn initialize_workspace(&self, root_uri: Url, cache_file: Option<PathBuf>) -> Result<()> {
        let root_path = uri_to_path(&root_uri)?;

        // Load or create the cache
        let cache = sync_cache(&root_path, cache_file.as_deref())?;

        let state = WorkspaceState { cache, cache_file };

        let mut workspaces = self.workspaces.write().await;
        workspaces.insert(root_uri, state);

        Ok(())
    }

    /// Get file ownership information for a specific file
    pub async fn get_file_ownership(&self, file_uri: &Url) -> Option<FileOwnershipInfo> {
        let file_path = uri_to_path(file_uri).ok()?;
        let workspaces = self.workspaces.read().await;

        // Find the workspace that contains this file
        for (root_uri, state) in workspaces.iter() {
            let root_path = uri_to_path(root_uri).ok()?;

            // Check if file is within this workspace
            if let Ok(relative_path) = file_path.strip_prefix(&root_path) {
                // Cache stores relative paths like "./main.go"
                let cache_path = PathBuf::from(".").join(relative_path);

                if let Some(file_entry) = state.cache.files.iter().find(|f| f.path == cache_path) {
                    return Some(FileOwnershipInfo {
                        path: relative_path.to_path_buf(),
                        owners: file_entry.owners.clone(),
                        tags: file_entry.tags.clone(),
                        is_unowned: file_entry.owners.is_empty()
                            || file_entry
                                .owners
                                .iter()
                                .any(|o| matches!(o.owner_type, OwnerType::Unowned)),
                    });
                }
            }
        }

        None
    }

    /// Refresh the cache for a workspace
    pub async fn refresh_workspace_cache(&self, root_uri: &Url) -> Result<()> {
        let root_path = uri_to_path(root_uri)?;
        let mut workspaces = self.workspaces.write().await;

        if let Some(state) = workspaces.get_mut(root_uri) {
            // Reload the cache
            state.cache = sync_cache(&root_path, state.cache_file.as_deref())?;
        }

        Ok(())
    }

    /// Publish diagnostics for unowned files in all workspaces
    pub async fn publish_unowned_diagnostics(&self) {
        let workspaces = self.workspaces.read().await;

        for (root_uri, state) in workspaces.iter() {
            let mut diagnostics = Vec::new();

            for file_entry in &state.cache.files {
                let is_unowned = file_entry.owners.is_empty()
                    || file_entry
                        .owners
                        .iter()
                        .any(|o| matches!(o.owner_type, OwnerType::Unowned));

                if is_unowned {
                    // Create a diagnostic for this unowned file
                    // We use a dummy position since we're reporting on the file itself
                    let file_path = root_uri.join(&file_entry.path.to_string_lossy().to_string());

                    if let Ok(file_uri) = file_path {
                        let diagnostic = Diagnostic {
                            range: Range {
                                start: Position::new(0, 0),
                                end: Position::new(0, 0),
                            },
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: Some(NumberOrString::String("unowned-file".to_string())),
                            source: Some("codeinput".to_string()),
                            message: "This file has no CODEOWNERS assignment".to_string(),
                            related_information: None,
                            tags: None,
                            code_description: None,
                            data: None,
                        };

                        diagnostics.push((file_uri, vec![diagnostic]));
                    }
                }
            }

            // Publish diagnostics for this workspace
            for (file_uri, file_diagnostics) in diagnostics {
                let Ok(ls_uri) = file_uri.as_str().parse::<Uri>() else { continue; };
                self.client
                    .publish_diagnostics(ls_uri, file_diagnostics, None)
                    .await;
            }
        }
    }
}

/// Convert a URL to a file path
pub fn uri_to_path(uri: &Url) -> Result<PathBuf> {
    uri.to_file_path()
        .map_err(|_| Error::new("Invalid file URI"))
}

/// Check if a URI points to a CODEOWNERS file
pub fn is_codeowners_file(uri: &Url) -> bool {
    let path = uri.path();
    path.contains("CODEOWNERS") || path.contains("codeowners")
}

/// Run the LSP server over stdio
pub async fn run_lsp_server() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = tower_lsp_server::LspService::new(|client| LspServer::new(client));

    tower_lsp_server::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;

    Ok(())
}

/// Run the LSP server over TCP
pub async fn run_lsp_server_tcp(port: u16) -> Result<()> {
    use tokio::net::TcpListener;

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await?;

    eprintln!("LSP server listening on {}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let (read, write) = tokio::io::split(stream);
        let (service, socket) = tower_lsp_server::LspService::new(|client| LspServer::new(client));

        tokio::spawn(tower_lsp_server::Server::new(read, write, socket).serve(service));
    }
}
