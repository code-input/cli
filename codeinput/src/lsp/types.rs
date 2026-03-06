use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::core::types::{Owner, Tag};

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

