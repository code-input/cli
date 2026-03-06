use serde::Serialize;
use serde_json::Value;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::LanguageServer;
use url::Url;

use super::server::{is_codeowners_file, uri_to_path, LspServer};

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
                inlay_hint_provider: Some(OneOf::Left(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "codeinput.listFiles".to_string(),
                        "codeinput.listOwners".to_string(),
                        "codeinput.listTags".to_string(),
                        "codeinput.getFileOwnership".to_string(),
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

    async fn inlay_hint(&self, params: InlayHintParams) -> LspResult<Option<Vec<InlayHint>>> {
        let file_uri = params.text_document.uri;

        let path = uri_to_path(&file_uri);
        let file_path = match path {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        if file_path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_lowercase() != "codeowners" {
            return Ok(None);
        }

        let workspaces = self.workspaces.read().await;

        for (_root_uri, state) in workspaces.iter() {
            let file_entries: Vec<_> = state.cache.entries.iter()
                .filter(|e| e.source_file == file_path)
                .collect();

            if file_entries.is_empty() {
                continue;
            }

            // Read the CODEOWNERS file content asynchronously to get line lengths
            let content = match tokio::fs::read_to_string(&file_path).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            let lines: Vec<&str> = content.lines().collect();
            let mut hints = vec![];

            for entry in file_entries {
                let line_idx = if entry.line_number > 0 { entry.line_number - 1 } else { 0 };
                let line_length = lines.get(line_idx).map(|l| l.len() as u32).unwrap_or(0);

                let matcher = crate::core::types::codeowners_entry_to_matcher(entry);
                
                let mut match_count = 0;
                for file_entry in &state.cache.files {
                    if matcher.override_matcher.matched(&file_entry.path, false).is_whitelist() {
                        match_count += 1;
                    }
                }

                hints.push(InlayHint {
                    position: Position::new(line_idx as u32, line_length),
                    label: InlayHintLabel::String(format!(
                        "  {} file{}",
                        match_count,
                        if match_count == 1 { "" } else { "s" }
                    )),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: Some(InlayHintTooltip::String(format!(
                        "This pattern matches {} file(s) in the repository",
                        match_count
                    ))),
                    padding_left: Some(true),
                    padding_right: Some(false),
                    data: None,
                });
            }

            if !hints.is_empty() {
                return Ok(Some(hints));
            }
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
            "codeinput.getFileOwnership" => {
                let uri_val = params.arguments.first().and_then(|v| v.as_str()).ok_or_else(|| {
                    tower_lsp::jsonrpc::Error::invalid_params("Expected a single URI string argument")
                })?;
                let result = self.get_file_ownership_command(uri_val.to_string()).await?;
                to_value(result).map(Some)
            }
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

