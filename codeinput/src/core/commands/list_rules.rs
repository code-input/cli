use crate::{
    core::{cache::sync_cache, display::truncate_string, types::OutputFormat},
    utils::error::{Error, Result},
};
use std::io::{self, Write};
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct RuleDisplay {
    #[tabled(rename = "Pattern")]
    pattern: String,
    #[tabled(rename = "Source")]
    source: String,
    #[tabled(rename = "Line")]
    line_number: usize,
    #[tabled(rename = "Owners")]
    owners: String,
    #[tabled(rename = "Tags")]
    tags: String,
}

/// Display CODEOWNERS rules from the cache
pub fn run(format: &OutputFormat, cache_file: Option<&std::path::Path>) -> Result<()> {
    // Load the cache
    let cache = sync_cache(std::path::Path::new("."), cache_file)?;

    // Process the rules from the cache
    match format {
        OutputFormat::Text => {
            // Create table data
            let table_data: Vec<RuleDisplay> = cache
                .entries
                .iter()
                .map(|entry| {
                    // Format owners list
                    let owners_display = if entry.owners.is_empty() {
                        "None".to_string()
                    } else {
                        entry
                            .owners
                            .iter()
                            .map(|o| o.identifier.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    // Format tags list
                    let tags_display = if entry.tags.is_empty() {
                        "None".to_string()
                    } else {
                        entry
                            .tags
                            .iter()
                            .map(|t| t.0.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    // Get source file name
                    let source_display = entry
                        .source_file
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| entry.source_file.to_string_lossy().to_string());

                    RuleDisplay {
                        pattern: truncate_string(&entry.pattern, 40),
                        source: truncate_string(&source_display, 20),
                        line_number: entry.line_number,
                        owners: truncate_string(&owners_display, 30),
                        tags: truncate_string(&tags_display, 25),
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
            println!("Total: {} rules", cache.entries.len());
        }
        OutputFormat::Json => {
            // Convert to a more friendly JSON structure
            let rules_data: Vec<_> = cache
                .entries
                .iter()
                .map(|entry| {
                    serde_json::json!({
                        "pattern": entry.pattern,
                        "source_file": entry.source_file.to_string_lossy().to_string(),
                        "line_number": entry.line_number,
                        "owners": entry.owners.iter().map(|o| {
                            serde_json::json!({
                                "identifier": o.identifier,
                                "type": o.owner_type.to_string()
                            })
                        }).collect::<Vec<_>>(),
                        "tags": entry.tags.iter().map(|t| &t.0).collect::<Vec<_>>()
                    })
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&rules_data).unwrap());
        }
        OutputFormat::Bincode => {
            let encoded =
                bincode::serde::encode_to_vec(&cache.entries, bincode::config::standard())
                    .map_err(|e| Error::new(&format!("Serialization error: {}", e)))?;

            // Write raw binary bytes to stdout
            io::stdout()
                .write_all(&encoded)
                .map_err(|e| Error::new(&format!("IO error: {}", e)))?;
        }
    }

    Ok(())
}
