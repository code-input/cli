use crate::{
    core::{
        cache::sync_cache,
        types::{CodeownersEntry, OutputFormat},
    },
    utils::error::{Error, Result},
};
use std::io::{self, Write};

/// Inspect ownership and tags for a specific file
pub fn run(
    file_path: &std::path::Path, repo: Option<&std::path::Path>, format: &OutputFormat,
    cache_file: Option<&std::path::Path>,
) -> Result<()> {
    // Repository path
    let repo = repo.unwrap_or_else(|| std::path::Path::new("."));

    // Load the cache
    let cache = sync_cache(repo, cache_file)?;

    // Normalize the file path to be relative to the repo
    let normalized_file_path = if file_path.is_absolute() {
        file_path
            .strip_prefix(repo)
            .map_err(|_| {
                Error::new(&format!(
                    "File {} is not within repository {}",
                    file_path.display(),
                    repo.display()
                ))
            })?
            .to_path_buf()
    } else {
        file_path.to_path_buf()
    };

    // Create normalized path for matching (handle ./ prefix variations)
    let path_str = normalized_file_path.to_string_lossy();
    let path_without_dot = path_str.strip_prefix("./").unwrap_or(&path_str);

    // Find the file in the cache (try both with and without ./ prefix)
    let file_entry = cache
        .files
        .iter()
        .find(|file| {
            let cache_path = file.path.to_string_lossy();
            let cache_path_normalized = cache_path.strip_prefix("./").unwrap_or(&cache_path);
            cache_path_normalized == path_without_dot
        })
        .ok_or_else(|| {
            Error::new(&format!(
                "File {} not found in cache",
                normalized_file_path.display()
            ))
        })?;

    // Find the CODEOWNERS entries that match this file
    let matching_entries: Vec<&CodeownersEntry> = cache
        .entries
        .iter()
        .filter(|entry| {
            // Simple pattern matching - in a real implementation you'd want proper glob matching
            let pattern = &entry.pattern;
            let file_str = normalized_file_path.to_string_lossy();

            if pattern.ends_with("*") {
                let prefix = &pattern[..pattern.len() - 1];
                file_str.starts_with(prefix)
            } else if let Some(suffix) = pattern.strip_prefix("*") {
                file_str.ends_with(suffix)
            } else if pattern.contains('*') {
                // Basic wildcard matching - could be improved
                let parts: Vec<&str> = pattern.split('*').collect();
                if parts.len() == 2 {
                    file_str.starts_with(parts[0]) && file_str.ends_with(parts[1])
                } else {
                    file_str == *pattern || file_str.starts_with(&format!("{}/", pattern))
                }
            } else {
                file_str == *pattern || file_str.starts_with(&format!("{}/", pattern))
            }
        })
        .collect();

    // Create inspection result
    let inspection_result = serde_json::json!({
        "file_path": normalized_file_path.to_string_lossy(),
        "owners": file_entry.owners,
        "tags": file_entry.tags.iter().map(|t| &t.0).collect::<Vec<_>>(),
        "matching_rules": matching_entries.iter().map(|entry| {
            serde_json::json!({
                "source_file": entry.source_file.to_string_lossy(),
                "line_number": entry.line_number,
                "pattern": entry.pattern,
                "owners": entry.owners,
                "tags": entry.tags.iter().map(|t| &t.0).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>()
    });

    // Output the inspection result in the requested format
    match format {
        OutputFormat::Text => {
            println!(
                "==============================================================================="
            );
            println!(" File: {}", normalized_file_path.display());
            println!(
                "==============================================================================="
            );
            println!("\nOwners:");
            if file_entry.owners.is_empty() {
                println!("  (no owners)");
            } else {
                for owner in &file_entry.owners {
                    println!("  - {}", owner.identifier);
                }
            }

            println!("\nTags:");
            if file_entry.tags.is_empty() {
                println!("  (no tags)");
            } else {
                for tag in &file_entry.tags {
                    println!("  - {}", tag.0);
                }
            }

            println!("\nMatching CODEOWNERS Rules:");
            if matching_entries.is_empty() {
                println!("  (no explicit rules)");
            } else {
                for entry in matching_entries {
                    println!(
                        "\n  From {}:{}",
                        entry.source_file.display(),
                        entry.line_number
                    );
                    println!("    Pattern: {}", entry.pattern);
                    let owners_str = entry
                        .owners
                        .iter()
                        .map(|o| o.identifier.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!("    Owners:  {}", owners_str);
                    if !entry.tags.is_empty() {
                        println!(
                            "    Tags:    {}",
                            entry
                                .tags
                                .iter()
                                .map(|t| t.0.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                }
            }
            println!();
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&inspection_result)
                    .map_err(|e| Error::new(&format!("JSON serialization error: {}", e)))?
            );
        }
        OutputFormat::Bincode => {
            let encoded =
                bincode::serde::encode_to_vec(&inspection_result, bincode::config::standard())
                    .map_err(|e| Error::new(&format!("Serialization error: {}", e)))?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| Error::new(&format!("IO error: {}", e)))?;
        }
    }

    Ok(())
}
