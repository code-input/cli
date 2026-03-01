use crate::{
    core::{
        cache::sync_cache,
        display::{truncate_path, truncate_string},
        types::OutputFormat,
    },
    utils::error::{Error, Result},
};
use std::io::{self, Write};
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct FileDisplay {
    #[tabled(rename = "File Path")]
    path: String,
    #[tabled(rename = "Owners")]
    owners: String,
    #[tabled(rename = "Tags")]
    tags: String,
}

/// Find and list files with their owners based on filter criteria
pub fn run(
    repo: Option<&std::path::Path>, tags: Option<&str>, owners: Option<&str>, unowned: bool,
    show_all: bool, format: &OutputFormat, cache_file: Option<&std::path::Path>,
) -> Result<()> {
    // Repository path
    let repo = repo.unwrap_or_else(|| std::path::Path::new("."));

    // Load the cache
    let cache = sync_cache(repo, cache_file)?;

    // Filter files based on criteria
    let filtered_files = cache
        .files
        .iter()
        .filter(|file| {
            // Check if we should include this file based on filters
            let passes_owner_filter = match owners {
                Some(owner_filter) => {
                    let owner_patterns: Vec<&str> = owner_filter.split(',').collect();
                    file.owners.iter().any(|owner| {
                        owner_patterns
                            .iter()
                            .any(|pattern| owner.identifier.contains(pattern))
                    })
                }
                None => true,
            };

            let passes_tag_filter = match tags {
                Some(tag_filter) => {
                    let tag_patterns: Vec<&str> = tag_filter.split(',').collect();
                    file.tags
                        .iter()
                        .any(|tag| tag_patterns.iter().any(|pattern| tag.0.contains(pattern)))
                }
                None => true,
            };

            let passes_unowned_filter = if unowned {
                file.owners.is_empty()
            } else {
                true
            };

            //  exclude unowned/untagged files unless show_all or unowned is specified
            let passes_ownership_requirement = if show_all || unowned {
                true
            } else {
                !file.owners.is_empty() || !file.tags.is_empty()
            };

            passes_owner_filter
                && passes_tag_filter
                && passes_unowned_filter
                && passes_ownership_requirement
        })
        .collect::<Vec<_>>();

    // Output the filtered files in the requested format
    match format {
        OutputFormat::Text => {
            // Create table data
            let table_data: Vec<FileDisplay> = filtered_files
                .iter()
                .map(|file| {
                    let path_str = file.path.to_string_lossy().to_string();

                    let owners_str = if file.owners.is_empty() {
                        "None".to_string()
                    } else {
                        file.owners
                            .iter()
                            .map(|o| o.identifier.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    let tags_str = if file.tags.is_empty() {
                        "None".to_string()
                    } else {
                        file.tags
                            .iter()
                            .map(|t| t.0.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    FileDisplay {
                        path: truncate_path(&path_str, 60),
                        owners: truncate_string(&owners_str, 40),
                        tags: truncate_string(&tags_str, 30),
                    }
                })
                .collect();

            // Get terminal width, fallback to 80 if unavailable
            let terminal_width =
                if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() {
                    w as usize
                } else {
                    80
                };

            let mut table = Table::new(table_data);
            table
                .with(tabled::settings::Style::modern())
                .with(tabled::settings::Width::wrap(
                    terminal_width.saturating_sub(4),
                ))
                .with(tabled::settings::Padding::new(1, 1, 0, 0));

            println!("{}", table);
            println!("Total: {} files", filtered_files.len());
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&filtered_files).unwrap());
        }
        OutputFormat::Bincode => {
            let encoded =
                bincode::serde::encode_to_vec(&filtered_files, bincode::config::standard())
                    .map_err(|e| Error::new(&format!("Serialization error: {}", e)))?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| Error::new(&format!("IO error: {}", e)))?;
        }
    }

    Ok(())
}
