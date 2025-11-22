use crate::{
    core::{
        cache::{build_cache, store_cache},
        common::{find_codeowners_files, find_files, get_repo_hash},
        parser::parse_codeowners,
        types::{CacheEncoding, CodeownersEntry},
    },
    utils::{app_config::AppConfig, error::Result},
};

/// Preprocess CODEOWNERS files and build ownership map
pub fn run(
    path: &std::path::Path, cache_file: Option<&std::path::Path>, encoding: CacheEncoding,
) -> Result<()> {
    println!("Parsing CODEOWNERS files at {}", path.display());

    let cache_file = match cache_file {
        Some(file) => path.join(file),
        None => {
            let config = AppConfig::fetch()?;
            path.join(config.cache_file)
        }
    };

    // Collect all CODEOWNERS files in the specified path
    let codeowners_files = find_codeowners_files(path)?;

    // Parse each CODEOWNERS file and collect entries
    let parsed_codeowners: Vec<CodeownersEntry> = codeowners_files
        .iter()
        .filter_map(|file| {
            let parsed = parse_codeowners(file).ok()?;
            Some(parsed)
        })
        .flatten()
        .collect();

    // Collect all files in the specified path
    let files = find_files(path)?;

    // Build the cache from the parsed CODEOWNERS entries and the files
    let hash = get_repo_hash(path)?;

    let cache = build_cache(parsed_codeowners, files, hash)?;

    // Store the cache in the specified file
    store_cache(&cache, &cache_file, encoding)?;

    Ok(())
}
