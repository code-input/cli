use crate::{
    core::{cache::sync_cache, display::truncate_string, types::OutputFormat},
    utils::error::{Error, Result},
};
use std::io::{self, Write};
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct OwnerDisplay {
    #[tabled(rename = "Owner")]
    identifier: String,
    #[tabled(rename = "Type")]
    owner_type: String,
    #[tabled(rename = "Files")]
    file_count: usize,
    #[tabled(rename = "Sample Files")]
    sample_files: String,
}

/// Display aggregated owner statistics and associations
pub fn run(
    repo: Option<&std::path::Path>, format: &OutputFormat, cache_file: Option<&std::path::Path>,
) -> Result<()> {
    // Repository path
    let repo = repo.unwrap_or_else(|| std::path::Path::new("."));

    // Load the cache
    let cache = sync_cache(repo, cache_file)?;

    // Sort owners by number of files they own (descending)
    let mut owners_with_counts: Vec<_> = cache.owners_map.iter().collect();
    owners_with_counts.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    // Process the owners from the cache
    match format {
        OutputFormat::Text => {
            // Create table data
            let table_data: Vec<OwnerDisplay> = owners_with_counts
                .iter()
                .map(|(owner, paths)| {
                    // Prepare sample file list
                    let file_samples = if paths.is_empty() {
                        "None".to_string()
                    } else {
                        let samples: Vec<_> = paths
                            .iter()
                            .take(3) // Show max 3 files as samples
                            .map(|p| {
                                let file_name = p
                                    .file_name()
                                    .map(|f| f.to_string_lossy().to_string())
                                    .unwrap_or_else(|| p.to_string_lossy().to_string());
                                file_name
                            })
                            .collect();
                        let mut display = samples.join(", ");
                        if paths.len() > 3 {
                            display.push_str(&format!(" (+{})", paths.len() - 3));
                        }
                        display
                    };

                    OwnerDisplay {
                        identifier: truncate_string(&owner.identifier, 35),
                        owner_type: format!("{:?}", owner.owner_type),
                        file_count: paths.len(),
                        sample_files: truncate_string(&file_samples, 45),
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
            println!("Total: {} owners", cache.owners_map.len());
        }
        OutputFormat::Json => {
            // Convert to a more friendly JSON structure
            let owners_data: Vec<_> = owners_with_counts.iter()
                .map(|(owner, paths)| {
                    serde_json::json!({
                        "identifier": owner.identifier,
                        "type": format!("{:?}", owner.owner_type),
                        "file_count": paths.len(),
                        "files": paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()
                    })
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&owners_data).unwrap());
        }
        OutputFormat::Bincode => {
            let encoded =
                bincode::serde::encode_to_vec(&owners_with_counts, bincode::config::standard())
                    .map_err(|e| Error::new(&format!("Serialization error: {}", e)))?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| Error::new(&format!("IO error: {}", e)))?;
        }
    }

    Ok(())
}
