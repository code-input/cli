use crate::core::{
    cache::load_cache,
    common::find_files,
    resolver::find_owners_and_tags_for_file,
    types::{CodeownersCache, Owner, OwnerType, codeowners_entry_to_matcher},
};
use crate::utils::error::{Error, Result};
use git2::{Blame, BlameOptions, Repository, Time};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use tabled::{Table, Tabled};

#[derive(Debug, Clone, PartialEq)]
pub enum InferScope {
    All,
    Unowned,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InferAlgorithm {
    Commits,
    Lines,
    Recent,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct FileOwnershipInference {
    pub file_path: PathBuf,
    pub inferred_owners: Vec<InferredOwner>,
    pub confidence: f64,
    pub existing_owners: Vec<Owner>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InferredOwner {
    pub email: String,
    pub username: Option<String>,
    pub score: f64,
    pub commits: u32,
    pub lines: u32,
    pub last_commit_days_ago: u32,
}

#[derive(Debug, Tabled)]
struct InferenceTableRow {
    #[tabled(rename = "File")]
    file: String,
    #[tabled(rename = "Current Owners")]
    current_owners: String,
    #[tabled(rename = "Inferred Owner")]
    inferred_owner: String,
    #[tabled(rename = "Score")]
    score: String,
    #[tabled(rename = "Commits")]
    commits: u32,
    #[tabled(rename = "Lines")]
    lines: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: Option<&Path>,
    scope: &InferScope,
    algorithm: &InferAlgorithm,
    lookback_days: u32,
    min_commits: u32,
    min_percentage: u32,
    cache_file: Option<&Path>,
    output_file: Option<&Path>,
) -> Result<()> {
    let base_path = path.unwrap_or_else(|| Path::new("."));
    let cache_path = cache_file.unwrap_or_else(|| Path::new(".codeowners.cache"));

    // Load existing cache if available
    let cache = match load_cache(cache_path) {
        Ok(cache) => Some(cache),
        Err(_) => {
            log::warn!("No cache found, running without CODEOWNERS context");
            None
        }
    };

    // Open git repository
    let repo = Repository::open(base_path)
        .map_err(|e| Error::with_source("Failed to open git repository", Box::new(e)))?;

    // Find files to analyze
    let files = find_files(base_path)?;
    let files_to_analyze = match scope {
        InferScope::All => files,
        InferScope::Unowned => filter_unowned_files(files, &cache)?,
    };

    log::info!("Analyzing {} files for ownership inference", files_to_analyze.len());

    // Analyze each file
    let mut inferences = Vec::new();
    for file_path in files_to_analyze {
        if let Ok(inference) = analyze_file_ownership(
            &repo,
            &file_path,
            base_path,
            algorithm,
            lookback_days,
            min_commits,
            min_percentage,
            &cache,
        ) {
            inferences.push(inference);
        }
    }

    // Output results
    if output_file.is_some() {
        output_codeowners(&inferences, output_file)?;
    } else {
        output_text(&inferences);
    }

    Ok(())
}

fn filter_unowned_files(
    files: Vec<PathBuf>,
    cache: &Option<CodeownersCache>,
) -> Result<Vec<PathBuf>> {
    let Some(cache) = cache else {
        return Ok(files);
    };

    let mut unowned_files = Vec::new();
    let matchers: Vec<_> = cache
        .entries
        .iter()
        .filter_map(|e| codeowners_entry_to_matcher(e).ok())
        .collect();
    for file in files {
        let (owners, _tags) = find_owners_and_tags_for_file(&file, &matchers)?;
        if owners.is_empty() || owners.iter().all(|o| o.owner_type == OwnerType::Unowned) {
            unowned_files.push(file);
        }
    }

    Ok(unowned_files)
}

#[allow(clippy::too_many_arguments)]
fn analyze_file_ownership(
    repo: &Repository,
    file_path: &Path,
    base_path: &Path,
    algorithm: &InferAlgorithm,
    lookback_days: u32,
    min_commits: u32,
    min_percentage: u32,
    cache: &Option<CodeownersCache>,
) -> Result<FileOwnershipInference> {
    // Get existing owners from cache
    let existing_owners = match cache {
        Some(cache) => {
            let matchers: Vec<_> = cache
                .entries
                .iter()
                .filter_map(|e| codeowners_entry_to_matcher(e).ok())
                .collect();
            let (owners, _tags) = find_owners_and_tags_for_file(file_path, &matchers).unwrap_or_default();
            owners
        },
        None => Vec::new(),
    };

    // Get git blame for the file
    let blame = get_file_blame(repo, file_path, base_path, lookback_days)?;

    // Analyze ownership based on algorithm
    let contributors = match algorithm {
        InferAlgorithm::Lines => analyze_by_lines(&blame, min_commits)?,
        InferAlgorithm::Commits => analyze_by_commits(repo, file_path, base_path, lookback_days, min_commits)?,
        InferAlgorithm::Recent => analyze_by_recent_activity(&blame, min_commits)?,
    };

    // Filter by minimum percentage
    let total_score: f64 = contributors.values().map(|c| c.score).sum();
    let min_score = (min_percentage as f64 / 100.0) * total_score;
    
    let mut inferred_owners: Vec<InferredOwner> = contributors
        .into_values()
        .filter(|c| c.score >= min_score)
        .collect();

    // Sort by score descending
    inferred_owners.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    // Calculate confidence (higher if fewer candidates, higher scores)
    let confidence = if inferred_owners.is_empty() {
        0.0
    } else {
        let top_score = inferred_owners[0].score;
        let score_ratio = if total_score > 0.0 { top_score / total_score } else { 0.0 };
        let candidate_penalty = 1.0 - (inferred_owners.len().min(5) as f64 * 0.1);
        (score_ratio * candidate_penalty).clamp(0.0, 1.0)
    };

    Ok(FileOwnershipInference {
        file_path: file_path.to_path_buf(),
        inferred_owners,
        confidence,
        existing_owners,
    })
}

fn get_file_blame<'a>(
    repo: &'a Repository,
    file_path: &Path,
    base_path: &Path,
    lookback_days: u32,
) -> Result<Blame<'a>> {
    let relative_path = file_path.strip_prefix(base_path)
        .map_err(|_| Error::new("File path is not within repository"))?;

    let mut blame_options = BlameOptions::new();
    
    // Set lookback period
    if lookback_days > 0 {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::days(lookback_days as i64);
        let _git_time = Time::new(cutoff_time.timestamp(), 0);
        blame_options.oldest_commit(repo.head()?.peel_to_commit()?.id());
        // Note: git2 doesn't have direct time filtering, so we'll handle this in analysis
    }

    repo.blame_file(relative_path, Some(&mut blame_options))
        .map_err(|e| Error::with_source("Failed to get git blame", Box::new(e)))
}

fn analyze_by_lines(blame: &Blame, min_commits: u32) -> Result<HashMap<String, InferredOwner>> {
    let mut contributors: HashMap<String, InferredOwner> = HashMap::new();

    for hunk in blame.iter() {
        let signature = hunk.final_signature();
        let email = signature.email().unwrap_or("unknown").to_string();
        
        let entry = contributors.entry(email.clone()).or_insert_with(|| InferredOwner {
            email: email.clone(),
            username: None,
            score: 0.0,
            commits: 0,
            lines: 0,
            last_commit_days_ago: u32::MAX,
        });

        entry.lines += hunk.lines_in_hunk() as u32;
        entry.score += hunk.lines_in_hunk() as f64;
        entry.commits += 1;

        // Update most recent commit
        let commit_time = hunk.final_signature().when();
        let days_ago = (chrono::Utc::now().timestamp() - commit_time.seconds()) / 86400;
        entry.last_commit_days_ago = entry.last_commit_days_ago.min(days_ago as u32);
    }

    // Filter by minimum commits
    contributors.retain(|_, contributor| contributor.commits >= min_commits);

    Ok(contributors)
}

fn analyze_by_commits(
    repo: &Repository,
    file_path: &Path,
    base_path: &Path,
    lookback_days: u32,
    min_commits: u32,
) -> Result<HashMap<String, InferredOwner>> {
    let relative_path = file_path.strip_prefix(base_path)
        .map_err(|_| Error::new("File path is not within repository"))?;

    let mut contributors: HashMap<String, InferredOwner> = HashMap::new();
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let cutoff_time = if lookback_days > 0 {
        Some(chrono::Utc::now() - chrono::Duration::days(lookback_days as i64))
    } else {
        None
    };

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        
        // Check time cutoff
        if let Some(cutoff) = cutoff_time {
            let commit_time = commit.time();
            if commit_time.seconds() < cutoff.timestamp() {
                break;
            }
        }

        // Check if commit touches our file
        if commit_touches_file(repo, &commit, relative_path)? {
            let signature = commit.author();
            let email = signature.email().unwrap_or("unknown").to_string();
            
            let entry = contributors.entry(email.clone()).or_insert_with(|| InferredOwner {
                email: email.clone(),
                username: None,
                score: 0.0,
                commits: 0,
                lines: 0,
                last_commit_days_ago: u32::MAX,
            });

            entry.commits += 1;
            entry.score += 1.0;

            let days_ago = (chrono::Utc::now().timestamp() - commit.time().seconds()) / 86400;
            entry.last_commit_days_ago = entry.last_commit_days_ago.min(days_ago as u32);
        }
    }

    contributors.retain(|_, contributor| contributor.commits >= min_commits);
    Ok(contributors)
}

fn analyze_by_recent_activity(blame: &Blame, min_commits: u32) -> Result<HashMap<String, InferredOwner>> {
    let mut contributors = analyze_by_lines(blame, min_commits)?;
    
    // Weight recent activity higher
    let _now = chrono::Utc::now().timestamp();
    for contributor in contributors.values_mut() {
        let days_ago = contributor.last_commit_days_ago as f64;
        let recency_weight = (1.0 / (1.0 + days_ago / 30.0)).max(0.1); // More weight for recent commits
        contributor.score *= recency_weight;
    }

    Ok(contributors)
}

fn commit_touches_file(repo: &Repository, commit: &git2::Commit, file_path: &Path) -> Result<bool> {
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    // repo is passed as parameter now
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    
    let file_path_str = file_path.to_str().ok_or_else(|| Error::new("Invalid file path"))?;
    let mut found = false;
    
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                if path.to_str() == Some(file_path_str) {
                    found = true;
                    return false; // Stop iteration
                }
            }
            if let Some(path) = delta.old_file().path() {
                if path.to_str() == Some(file_path_str) {
                    found = true;
                    return false; // Stop iteration  
                }
            }
            true // Continue iteration
        },
        None,
        None,
        None,
    )?;

    Ok(found)
}


fn output_text(inferences: &[FileOwnershipInference]) {
    if inferences.is_empty() {
        println!("No ownership inferences found.");
        return;
    }

    let mut rows = Vec::new();
    for inference in inferences {
        let current_owners = if inference.existing_owners.is_empty() {
            "None".to_string()
        } else {
            inference.existing_owners.iter()
                .map(|o| format!("{:?}", o))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let (inferred_owner, score, commits, lines) = if let Some(top_owner) = inference.inferred_owners.first() {
            (
                top_owner.email.clone(),
                format!("{:.1}%", top_owner.score * 100.0),
                top_owner.commits,
                top_owner.lines,
            )
        } else {
            ("None".to_string(), "0%".to_string(), 0, 0)
        };

        rows.push(InferenceTableRow {
            file: inference.file_path.display().to_string(),
            current_owners,
            inferred_owner,
            score,
            commits,
            lines,
        });
    }

    let table = Table::new(rows);
    println!("{}", table);
    
    println!("\nSummary:");
    println!("  Total files analyzed: {}", inferences.len());
    println!("  Files with inferred owners: {}", 
        inferences.iter().filter(|i| !i.inferred_owners.is_empty()).count());
    println!("  Average confidence: {:.1}%", 
        inferences.iter().map(|i| i.confidence).sum::<f64>() / inferences.len() as f64 * 100.0);
}


fn output_codeowners(inferences: &[FileOwnershipInference], output_file: Option<&Path>) -> Result<()> {
    let mut output_lines = Vec::new();

    for inference in inferences {
        if let Some(top_owner) = inference.inferred_owners.first() {
            let owner_str = top_owner.email.clone();
            let pattern = inference.file_path.display().to_string();
            output_lines.push(format!("{} {}", pattern, owner_str));
        }
    }

    if let Some(file_path) = output_file {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .map_err(|e| Error::with_source(&format!("Failed to open file: {}", file_path.display()), Box::new(e)))?;
        
        for line in &output_lines {
            writeln!(file, "{}", line)
                .map_err(|e| Error::with_source("Failed to write to file", Box::new(e)))?;
        }
        
        log::info!("Appended {} CODEOWNERS entries to {}", 
            inferences.iter().filter(|i| !i.inferred_owners.is_empty()).count(),
            file_path.display());
    }

    Ok(())
}