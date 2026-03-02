use crate::utils::{
    error::Result,
    output,
};

use super::{
    cache::{build_cache, store_cache},
    common::{find_codeowners_files, find_files, get_repo_hash},
    parser::parse_codeowners,
    types::{CacheEncoding, CodeownersCache, CodeownersEntry},
};

pub fn parse_repo(repo: &std::path::Path, cache_file: &std::path::Path) -> Result<CodeownersCache> {
    output::println(&format!("Parsing CODEOWNERS files at {}", repo.display()));

    // Collect all CODEOWNERS files in the specified path
    let codeowners_files = find_codeowners_files(repo)?;

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
    let files = find_files(repo)?;

    // Get the hash of the repository
    let hash = get_repo_hash(repo)?;

    // Build the cache from the parsed CODEOWNERS entries and the files
    let cache = build_cache(parsed_codeowners, files, hash)?;

    // Store the cache in the specified file
    store_cache(&cache, &repo.join(cache_file), CacheEncoding::Bincode)?;

    output::println("CODEOWNERS parsing completed successfully");

    Ok(cache)
}
