//! LSP Server implementation for CODEOWNERS integration
//!
//! Provides LSP handlers for:
//! - textDocument/hover: Show owners/tags when hovering over file paths
//! - textDocument/codeLens: Display ownership information above files
//! - textDocument/publishDiagnostics: Warn about unowned files
//! - Custom methods:
//!   - codeinput/listFiles: List all files with ownership info
//!   - codeinput/listOwners: List all owners with their files
//!   - codeinput/listTags: List all tags with their files

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use url::Url;

use crate::core::cache::sync_cache;
use crate::core::types::{CodeownersCache, Owner, OwnerType, Tag};
use crate::utils::error::{Error, Result};

/// LSP Server state
pub struct LspServer {
    client: Client,
    /// Map of workspace root URIs to their cached CODEOWNERS data
    workspaces: Arc<RwLock<HashMap<Url, WorkspaceState>>>,
}

/// State for a single workspace
#[derive(Debug)]
struct WorkspaceState {
    cache: CodeownersCache,
    cache_file: Option<PathBuf>,
}

/// Information about a file's ownership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOwnershipInfo {
    pub path: PathBuf,
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
    pub is_unowned: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListFilesResponse {
    pub files: Vec<FileOwnershipInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OwnerInfo {
    pub owner: Owner,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListOwnersResponse {
    pub owners: Vec<OwnerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagInfo {
    pub tag: Tag,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTagsResponse {
    pub tags: Vec<TagInfo>,
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
    async fn initialize_workspace(&self, root_uri: Url, cache_file: Option<PathBuf>) -> Result<()> {
        let root_path = uri_to_path(&root_uri)?;

        // Load or create the cache
        let cache = sync_cache(&root_path, cache_file.as_deref())?;

        let state = WorkspaceState { cache, cache_file };

        let mut workspaces = self.workspaces.write().await;
        workspaces.insert(root_uri, state);

        Ok(())
    }

    /// Get file ownership information for a specific file
    async fn get_file_ownership(&self, file_uri: &Url) -> Option<FileOwnershipInfo> {
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
    async fn refresh_workspace_cache(&self, root_uri: &Url) -> Result<()> {
        let root_path = uri_to_path(root_uri)?;
        let mut workspaces = self.workspaces.write().await;

        if let Some(state) = workspaces.get_mut(root_uri) {
            // Reload the cache
            state.cache = sync_cache(&root_path, state.cache_file.as_deref())?;
        }

        Ok(())
    }

    /// Publish diagnostics for unowned files in all workspaces
    async fn publish_unowned_diagnostics(&self) {
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
                self.client
                    .publish_diagnostics(file_uri, file_diagnostics, None)
                    .await;
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LspServer {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        // Initialize workspaces from workspace folders
        let workspace_folders = params.workspace_folders.unwrap_or_default();

        for folder in workspace_folders {
            if let Err(e) = self.initialize_workspace(folder.uri, None).await {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Failed to initialize workspace: {}", e),
                    )
                    .await;
            }
        }

        // If no workspace folders, try to use root_uri
        if let Some(root_uri) = params.root_uri {
            if let Err(e) = self.initialize_workspace(root_uri, None).await {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Failed to initialize root workspace: {}", e),
                    )
                    .await;
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "codeinput.listFiles".to_string(),
                        "codeinput.listOwners".to_string(),
                        "codeinput.listTags".to_string(),
                    ],
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                }),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "codeinput-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "codeinput LSP server initialized")
            .await;

        // Publish initial diagnostics
        self.publish_unowned_diagnostics().await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let file_uri = params.text_document.uri;

        // Check if this is a CODEOWNERS file and refresh cache if so
        if is_codeowners_file(&file_uri) {
            // Find which workspace this file belongs to
            let matching_root = {
                let workspaces = self.workspaces.read().await;
                workspaces
                    .keys()
                    .find(|root_uri| file_uri.as_str().starts_with(root_uri.as_str()))
                    .cloned()
            };

            if let Some(root_uri) = matching_root {
                if let Err(e) = self.refresh_workspace_cache(&root_uri).await {
                    self.client
                        .log_message(
                            MessageType::WARNING,
                            format!("Failed to refresh cache: {}", e),
                        )
                        .await;
                }
                // Re-publish diagnostics after cache refresh
                self.publish_unowned_diagnostics().await;
            }
        }
    }

    async fn did_change(&self, _params: DidChangeTextDocumentParams) {
        // We use full sync, so we don't need to handle incremental changes
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let file_uri = params.text_document.uri;

        // If CODEOWNERS file was saved, refresh the cache
        if is_codeowners_file(&file_uri) {
            let matching_root = {
                let workspaces = self.workspaces.read().await;
                workspaces
                    .keys()
                    .find(|root_uri| file_uri.as_str().starts_with(root_uri.as_str()))
                    .cloned()
            };

            if let Some(root_uri) = matching_root {
                if let Err(e) = self.refresh_workspace_cache(&root_uri).await {
                    self.client
                        .log_message(
                            MessageType::WARNING,
                            format!("Failed to refresh cache: {}", e),
                        )
                        .await;
                }
                self.publish_unowned_diagnostics().await;
            }
        }
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let file_uri = params.text_document_position_params.text_document.uri;

        // Get ownership info for this file
        if let Some(info) = self.get_file_ownership(&file_uri).await {
            let mut contents = vec![];

            // Add owners section
            if info.owners.is_empty() {
                contents.push(MarkedString::String("**Owners:** (none)".to_string()));
            } else {
                let owners_str = info
                    .owners
                    .iter()
                    .map(|o| format!("`{}`", o.identifier))
                    .collect::<Vec<_>>()
                    .join(", ");
                contents.push(MarkedString::String(format!("**Owners:** {}", owners_str)));
            }

            // Add tags section
            if !info.tags.is_empty() {
                let tags_str = info
                    .tags
                    .iter()
                    .map(|t| format!("`#{}`", t.0))
                    .collect::<Vec<_>>()
                    .join(", ");
                contents.push(MarkedString::String(format!("**Tags:** {}", tags_str)));
            }

            // Add warning if unowned
            if info.is_unowned {
                contents.push(MarkedString::String(
                    "⚠️ **Warning:** This file has no CODEOWNERS assignment".to_string(),
                ));
            }

            return Ok(Some(Hover {
                contents: HoverContents::Array(contents),
                range: None,
            }));
        }

        Ok(None)
    }

    async fn code_lens(&self, params: CodeLensParams) -> LspResult<Option<Vec<CodeLens>>> {
        let file_uri = params.text_document.uri;

        // Get ownership info for this file
        if let Some(info) = self.get_file_ownership(&file_uri).await {
            let mut lenses = vec![];

            // Create a CodeLens showing ownership at the top of the file
            if !info.owners.is_empty() {
                let owners_str = info
                    .owners
                    .iter()
                    .map(|o| o.identifier.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                // Safely serialize arguments
                let args = (
                    serde_json::to_value(file_uri.to_string()).ok(),
                    serde_json::to_value(&info.owners).ok(),
                );
                if let (Some(uri_val), Some(owners_val)) = args {
                    lenses.push(CodeLens {
                        range: Range {
                            start: Position::new(0, 0),
                            end: Position::new(0, 0),
                        },
                        command: Some(Command {
                            title: format!("$(organization)  {}", owners_str),
                            command: "codeinput.showOwners".to_string(),
                            arguments: Some(vec![uri_val, owners_val]),
                        }),
                        data: None,
                    });
                }
            }

            // Add tags CodeLens if any
            if !info.tags.is_empty() {
                let tags_str = info
                    .tags
                    .iter()
                    .map(|t| format!("#{}", t.0))
                    .collect::<Vec<_>>()
                    .join(", ");

                // Safely serialize arguments
                let args = (
                    serde_json::to_value(file_uri.to_string()).ok(),
                    serde_json::to_value(&info.tags).ok(),
                );
                if let (Some(uri_val), Some(tags_val)) = args {
                    lenses.push(CodeLens {
                        range: Range {
                            start: Position::new(0, 0),
                            end: Position::new(0, 0),
                        },
                        command: Some(Command {
                            title: format!("$(tag)  {}", tags_str),
                            command: "codeinput.showTags".to_string(),
                            arguments: Some(vec![uri_val, tags_val]),
                        }),
                        data: None,
                    });
                }
            }

            // Add unowned warning CodeLens
            if info.is_unowned {
                // Safely serialize arguments
                if let Some(uri_val) = serde_json::to_value(file_uri.to_string()).ok() {
                    lenses.push(CodeLens {
                        range: Range {
                            start: Position::new(0, 0),
                            end: Position::new(0, 0),
                        },
                        command: Some(Command {
                            title: "$(warning)  Unowned file".to_string(),
                            command: "codeinput.addOwner".to_string(),
                            arguments: Some(vec![uri_val]),
                        }),
                        data: None,
                    });
                }
            }

            return Ok(Some(lenses));
        }

        Ok(None)
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        // Handle removed workspaces - collect URIs first, then remove them
        let removed_uris: Vec<Url> = params.event.removed.iter().map(|f| f.uri.clone()).collect();
        {
            let mut workspaces = self.workspaces.write().await;
            for uri in removed_uris {
                workspaces.remove(&uri);
            }
        }

        // Handle added workspaces
        for folder in params.event.added {
            if let Err(e) = self.initialize_workspace(folder.uri, None).await {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Failed to initialize workspace: {}", e),
                    )
                    .await;
            }
        }
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> LspResult<Option<Value>> {
        fn to_value<T: Serialize>(v: T) -> LspResult<Value> {
            serde_json::to_value(v)
                .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(e.to_string()))
        }

        match params.command.as_str() {
            "codeinput.listFiles" => {
                let result = self.list_files(None).await?;
                to_value(result).map(Some)
            }
            "codeinput.listOwners" => {
                let result = self.list_owners(None).await?;
                to_value(result).map(Some)
            }
            "codeinput.listTags" => {
                let result = self.list_tags(None).await?;
                to_value(result).map(Some)
            }
            _ => Err(tower_lsp::jsonrpc::Error::method_not_found()),
        }
    }
}

impl LspServer {
    pub async fn list_files(&self, workspace_uri: Option<Url>) -> LspResult<ListFilesResponse> {
        let workspaces = self.workspaces.read().await;

        let files = if let Some(uri) = workspace_uri {
            if let Some(state) = workspaces.get(&uri) {
                Self::collect_files_from_cache(&state.cache)
            } else {
                Vec::new()
            }
        } else {
            let mut all_files = Vec::new();
            for state in workspaces.values() {
                all_files.extend(Self::collect_files_from_cache(&state.cache));
            }
            all_files
        };

        Ok(ListFilesResponse { files })
    }

    pub async fn list_owners(&self, workspace_uri: Option<Url>) -> LspResult<ListOwnersResponse> {
        let workspaces = self.workspaces.read().await;

        let owners = if let Some(uri) = workspace_uri {
            if let Some(state) = workspaces.get(&uri) {
                Self::collect_owners_from_cache(&state.cache)
            } else {
                Vec::new()
            }
        } else {
            let mut all_owners = Vec::new();
            for state in workspaces.values() {
                all_owners.extend(Self::collect_owners_from_cache(&state.cache));
            }
            all_owners
        };

        Ok(ListOwnersResponse { owners })
    }

    pub async fn list_tags(&self, workspace_uri: Option<Url>) -> LspResult<ListTagsResponse> {
        let workspaces = self.workspaces.read().await;

        let tags = if let Some(uri) = workspace_uri {
            if let Some(state) = workspaces.get(&uri) {
                Self::collect_tags_from_cache(&state.cache)
            } else {
                Vec::new()
            }
        } else {
            let mut all_tags = Vec::new();
            for state in workspaces.values() {
                all_tags.extend(Self::collect_tags_from_cache(&state.cache));
            }
            all_tags
        };

        Ok(ListTagsResponse { tags })
    }

    fn collect_files_from_cache(cache: &CodeownersCache) -> Vec<FileOwnershipInfo> {
        cache
            .files
            .iter()
            .map(|entry| FileOwnershipInfo {
                path: entry.path.clone(),
                owners: entry.owners.clone(),
                tags: entry.tags.clone(),
                is_unowned: entry.owners.is_empty()
                    || entry
                        .owners
                        .iter()
                        .any(|o| matches!(o.owner_type, OwnerType::Unowned)),
            })
            .collect()
    }

    fn collect_owners_from_cache(cache: &CodeownersCache) -> Vec<OwnerInfo> {
        cache
            .owners_map
            .iter()
            .map(|(owner, files)| OwnerInfo {
                owner: owner.clone(),
                files: files.clone(),
            })
            .collect()
    }

    fn collect_tags_from_cache(cache: &CodeownersCache) -> Vec<TagInfo> {
        cache
            .tags_map
            .iter()
            .map(|(tag, files)| TagInfo {
                tag: tag.clone(),
                files: files.clone(),
            })
            .collect()
    }
}

/// Convert a URL to a file path
fn uri_to_path(uri: &Url) -> Result<PathBuf> {
    uri.to_file_path()
        .map_err(|_| Error::new("Invalid file URI"))
}

/// Check if a URI points to a CODEOWNERS file
fn is_codeowners_file(uri: &Url) -> bool {
    let path = uri.path();
    path.contains("CODEOWNERS") || path.contains("codeowners")
}

/// Run the LSP server over stdio
pub async fn run_lsp_server() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = tower_lsp::LspService::new(|client| LspServer::new(client));

    tower_lsp::Server::new(stdin, stdout, socket)
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
        let (service, socket) = tower_lsp::LspService::new(|client| LspServer::new(client));

        tokio::spawn(tower_lsp::Server::new(read, write, socket).serve(service));
    }
}
