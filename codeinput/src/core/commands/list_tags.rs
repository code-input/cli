use crate::{
    core::{cache::sync_cache, display::truncate_string, types::OutputFormat},
    utils::error::{Error, Result},
};
use std::io::{self, Write};
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct TagDisplay {
    #[tabled(rename = "Tag")]
    name: String,
    #[tabled(rename = "Files")]
    file_count: usize,
    #[tabled(rename = "Sample Files")]
    sample_files: String,
}

/// Audit and analyze tag usage across CODEOWNERS files
pub fn run(
    repo: Option<&std::path::Path>, format: &OutputFormat, cache_file: Option<&std::path::Path>,
) -> Result<()> {
    // Repository path
    let repo = repo.unwrap_or_else(|| std::path::Path::new("."));

    // Load the cache
    let cache = sync_cache(repo, cache_file)?;

    // Sort tags by number of files they're associated with (descending)
    let mut tags_with_counts: Vec<_> = cache.tags_map.iter().collect();
    tags_with_counts.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    // Process the tags from the cache
    match format {
        OutputFormat::Text => {
            // Create table data
            let table_data: Vec<TagDisplay> = tags_with_counts
                .iter()
                .map(|(tag, paths)| {
                    // Prepare sample file list - show filenames only, not full paths
                    let file_samples = if paths.is_empty() {
                        "None".to_string()
                    } else {
                        let samples: Vec<_> = paths
                            .iter()
                            .take(5) // Show max 5 files as samples
                            .map(|p| {
                                p.file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_else(|| p.to_string_lossy().to_string())
                            })
                            .collect();

                        let mut display = samples.join(", ");
                        if paths.len() > 5 {
                            display.push_str(&format!(" (+{})", paths.len() - 5));
                        }
                        display
                    };

                    TagDisplay {
                        name: truncate_string(&tag.0, 30),
                        file_count: paths.len(),
                        sample_files: truncate_string(&file_samples, 60),
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
            println!("Total: {} tags", cache.tags_map.len());
        }
        OutputFormat::Json => {
            // Convert to a more friendly JSON structure
            let tags_data: Vec<_> = tags_with_counts.iter()
                .map(|(tag, paths)| {
                    serde_json::json!({
                        "name": tag.0,
                        "file_count": paths.len(),
                        "files": paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()
                    })
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&tags_data).unwrap());
        }
        OutputFormat::Bincode => {
            let encoded =
                bincode::serde::encode_to_vec(&tags_with_counts, bincode::config::standard())
                    .map_err(|e| Error::new(&format!("Serialization error: {}", e)))?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| Error::new(&format!("IO error: {}", e)))?;
        }
    }

    Ok(())
}
