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

        self.client
            .log_message(
                MessageType::INFO,
                format!("inlay_hint called for: {}", file_uri),
            )
            .await;

        // Only show inlay hints in CODEOWNERS files
        let path = uri_to_path(&file_uri);
        if let Ok(path) = path {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default();
            self.client
                .log_message(MessageType::INFO, format!("file_name: {}", file_name))
                .await;
            if file_name.to_lowercase() != "codeowners" {
                self.client
                    .log_message(MessageType::INFO, "Not a CODEOWNERS file, skipping")
                    .await;
                return Ok(None);
            }
        } else {
            self.client
                .log_message(MessageType::WARNING, "Failed to convert URI to path")
                .await;
            return Ok(None);
        }

        let workspaces = self.workspaces.read().await;
        self.client
            .log_message(
                MessageType::INFO,
                format!("Workspace count: {}", workspaces.len()),
            )
            .await;

        // Find the workspace that contains this file
        for (root_uri, state) in workspaces.iter() {
            self.client
                .log_message(
                    MessageType::INFO,
                    format!("Checking workspace: {}", root_uri),
                )
                .await;
            let mut hints = vec![];

            let file_path = match uri_to_path(&file_uri) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Read the CODEOWNERS file content
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("Read CODEOWNERS file, {} lines", content.lines().count()),
                    )
                    .await;
                for (line_num, line) in content.lines().enumerate() {
                    let trimmed = line.trim();

                    // Skip empty lines and comments
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }

                    // Parse pattern from line
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if !parts.is_empty() {
                        let pattern = parts[0];

                        // Count files matching this pattern using the cache
                        let match_count = state
                            .cache
                            .entries
                            .iter()
                            .filter(|entry| {
                                let rel_path = entry.source_file.to_string_lossy();
                                // Simple pattern matching
                                if pattern == "*" {
                                    return true;
                                }
                                if pattern.ends_with('/') {
                                    let dir_pattern = &pattern[..pattern.len() - 1];
                                    rel_path.starts_with(dir_pattern)
                                } else if pattern.starts_with('*') {
                                    let suffix = &pattern[1..];
                                    rel_path.ends_with(suffix)
                                } else if pattern.ends_with("/*") {
                                    let prefix = &pattern[..pattern.len() - 2];
                                    rel_path.starts_with(prefix)
                                } else {
                                    rel_path.contains(pattern.trim_start_matches('/'))
                                }
                            })
                            .count();

                        if match_count > 0 {
                            let line_length = line.len() as u32;
                            hints.push(InlayHint {
                                position: Position::new(line_num as u32, line_length),
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
                    }
                }

                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("Generated {} inlay hints", hints.len()),
                    )
                    .await;
                return Ok(Some(hints));
            }
        }

        self.client
            .log_message(
                MessageType::WARNING,
                "No workspace found for CODEOWNERS file",
            )
            .await;
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

