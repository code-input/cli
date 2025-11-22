use crate::utils::error::{Error, Result};
use git2::{DiffFormat, DiffOptions, Repository};
use ignore::Walk;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use super::types::CodeownersEntry;

/// Maximum recursion depth for finding CODEOWNERS files (prevents stack overflow on deep structures)
const MAX_RECURSION_DEPTH: usize = 100;

/// Find CODEOWNERS files recursively in the given directory and its subdirectories
pub fn find_codeowners_files<P: AsRef<Path>>(base_path: P) -> Result<Vec<PathBuf>> {
    find_codeowners_files_impl(base_path.as_ref(), 0)
}

fn find_codeowners_files_impl(base_path: &Path, depth: usize) -> Result<Vec<PathBuf>> {
    if depth > MAX_RECURSION_DEPTH {
        log::warn!(
            "Maximum recursion depth ({}) reached while searching for CODEOWNERS files at {}",
            MAX_RECURSION_DEPTH,
            base_path.display()
        );
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == "CODEOWNERS")
                    .unwrap_or(false)
            {
                result.push(path);
            } else if path.is_dir() {
                result.extend(find_codeowners_files_impl(&path, depth + 1)?);
            }
        }
    }

    Ok(result)
}

/// Find all files in the given directory and its subdirectories
pub fn find_files<P: AsRef<Path>>(base_path: P) -> Result<Vec<PathBuf>> {
    let result = Walk::new(base_path)
        .filter_map(|entry| entry.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| e.clone().file_name().to_str().unwrap() != "CODEOWNERS")
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();

    Ok(result)
}


/// Files to exclude from the repository hash calculation (these are generated files)
const HASH_EXCLUDED_PATTERNS: &[&str] = &[
    ".codeowners.cache",
    "*.codeowners.cache",
];

pub fn get_repo_hash(repo_path: &Path) -> Result<[u8; 32]> {
    let repo = Repository::open(repo_path)
        .map_err(|e| Error::with_source("Failed to open repo", Box::new(e)))?;

    // 1. Get HEAD commit hash (or zeros if unborn)
    let head_oid = repo
        .head()
        .and_then(|r| r.resolve())
        .map(|r| r.target())
        .unwrap_or(None);

    // 2. Get index/staging area tree hash
    let mut index = repo
        .index()
        .map_err(|e| Error::with_source("Failed to get index", Box::new(e)))?;

    let index_tree = index
        .write_tree()
        .map_err(|e| Error::with_source("Failed to write index tree", Box::new(e)))?;

    // 3. Calculate hash of unstaged changes, excluding cache files
    let unstaged_hash = {
        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        // Add pathspec exclusions for cache files
        for pattern in HASH_EXCLUDED_PATTERNS {
            diff_opts.pathspec(format!(":(exclude){}", pattern));
        }

        let diff = repo
            .diff_index_to_workdir(None, Some(&mut diff_opts))
            .map_err(|e| Error::with_source("Failed to get diff", Box::new(e)))?;

        let mut hasher = Sha256::new();
        diff.print(DiffFormat::Patch, |delta, _, line| {
            // Double-check: skip any cache files that might have slipped through
            if let Some(path) = delta.new_file().path() {
                let path_str = path.to_string_lossy();
                if HASH_EXCLUDED_PATTERNS.iter().any(|p| {
                    if p.starts_with('*') {
                        path_str.ends_with(&p[1..])
                    } else {
                        path_str == *p
                    }
                }) {
                    return true; // Skip this file
                }
            }
            hasher.update(line.content());
            true
        })
        .map_err(|e| Error::with_source("Failed to print diff", Box::new(e)))?;
        hasher.finalize()
    };

    // 4. Combine all components into final hash
    let mut hasher = Sha256::new();
    hasher.update(head_oid.unwrap_or(git2::Oid::zero()).as_bytes());
    hasher.update(index_tree.as_bytes());
    hasher.update(&unstaged_hash);

    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_find_codeowners_files() -> Result<()> {
        // Create a temporary directory structure
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path();

        // Create test directory structure
        let sub_dir = base_path.join("subdir");
        let nested_dir = sub_dir.join("nested");
        fs::create_dir_all(&nested_dir)?;

        // Create CODEOWNERS files in different locations
        File::create(base_path.join("CODEOWNERS"))?;
        File::create(nested_dir.join("CODEOWNERS"))?;

        // Create some other files to verify we don't pick them up
        File::create(base_path.join("codeowners"))?; // wrong case
        File::create(sub_dir.join("not_codeowners"))?;

        // Find all CODEOWNERS files
        let found_files = find_codeowners_files(base_path)?;

        // Verify results
        assert_eq!(found_files.len(), 2);
        assert!(found_files
            .iter()
            .any(|p| p == &base_path.join("CODEOWNERS")));
        assert!(found_files
            .iter()
            .any(|p| p == &nested_dir.join("CODEOWNERS")));

        Ok(())
    }

    #[test]
    fn test_find_codeowners_files_empty_dir() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let found_files = find_codeowners_files(temp_dir.path())?;
        assert!(found_files.is_empty());
        Ok(())
    }

    #[test]
    fn test_find_codeowners_files_nonexistent_dir() -> Result<()> {
        let nonexistent_dir = PathBuf::from("/nonexistent/directory");
        let found_files = find_codeowners_files(nonexistent_dir)?;
        assert!(found_files.is_empty());
        Ok(())
    }
}
