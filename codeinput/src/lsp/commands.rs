use tower_lsp::jsonrpc::Result as LspResult;
use url::Url;

use crate::core::types::{CodeownersCache, OwnerType};

use super::server::LspServer;
use super::types::*;

impl LspServer {
    pub async fn get_file_ownership_command(&self, uri_str: String) -> LspResult<Option<FileOwnershipInfo>> {
        let Ok(uri) = Url::parse(&uri_str) else {
            return Err(tower_lsp::jsonrpc::Error::invalid_params("Invalid URI"));
        };
        Ok(self.get_file_ownership(&uri).await)
    }

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


