use std::path::PathBuf;

#[cfg(feature = "ignore")]
use ignore::overrides::Override;
use serde::{Deserialize, Serialize};

/// Normalizes a CODEOWNERS pattern to match GitHub's behavior
///
/// GitHub CODEOWNERS directory matching rules:
/// - `/path/to/dir/` matches all files and subdirectories under that path (converted to `/path/to/dir/**`)
/// - `/path/to/dir/*` matches direct files only (kept as-is)
/// - `/path/to/dir/**` matches everything recursively (kept as-is)
/// - Other patterns are kept as-is
fn normalize_codeowners_pattern(pattern: &str) -> String {
    // If pattern ends with `/` but not `*/` or `**/`, convert to `/**`
    if pattern.ends_with('/') && !pattern.ends_with("*/") && !pattern.ends_with("**/") {
        format!("{}**", pattern)
    } else {
        pattern.to_string()
    }
}

/// A parsed CODEOWNERS entry representing a single ownership rule.
///
/// Each entry corresponds to a line in a CODEOWNERS file that defines
/// which teams or individuals own files matching a specific pattern.
///
/// # Fields
///
/// * `source_file` - Path to the CODEOWNERS file containing this entry
/// * `line_number` - Line number (0-indexed) where this entry appears
/// * `pattern` - The glob pattern for matching files (e.g., `*.rs`, `/docs/`)
/// * `owners` - List of owners assigned to files matching this pattern
/// * `tags` - Optional metadata tags (e.g., `#backend`, `#critical`)
///
/// # Example
///
/// A CODEOWNERS line like `*.rs @rust-team #backend` would produce:
/// - `pattern`: `"*.rs"`
/// - `owners`: `[Owner { identifier: "@rust-team", owner_type: Team }]`
/// - `tags`: `[Tag("backend")]`
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeownersEntry {
    pub source_file: PathBuf,
    pub line_number: usize,
    pub pattern: String,
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
}

/// An inline CODEOWNERS declaration embedded within a source file.
///
/// Inline declarations allow files to specify their own ownership using
/// a special marker comment: `!!!CODEOWNERS @owner1 @owner2 #tag1`
///
/// This is useful when a specific file requires different ownership
/// than what the main CODEOWNERS file would assign.
///
/// # Example
///
/// A Rust file containing `// !!!CODEOWNERS @security-team #critical` would
/// produce an entry with the security team as owner regardless of the
/// patterns in the root CODEOWNERS file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCodeownersEntry {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
}

/// CODEOWNERS entry with Override matcher
#[cfg(feature = "ignore")]
#[derive(Debug)]
pub struct CodeownersEntryMatcher {
    pub source_file: PathBuf,
    pub line_number: usize,
    pub pattern: String,
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
    pub override_matcher: Override,
}

/// Error type for pattern matching failures
#[derive(Debug)]
pub struct PatternError {
    pub pattern: String,
    pub source_file: PathBuf,
    pub message: String,
}

impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid pattern '{}' in {}: {}",
            self.pattern,
            self.source_file.display(),
            self.message
        )
    }
}

impl std::error::Error for PatternError {}

/// Converts a CodeownersEntry to a CodeownersEntryMatcher with pattern compilation.
///
/// # Errors
///
/// Returns a `PatternError` if:
/// - The entry's source file has no parent directory
/// - The pattern is invalid and cannot be compiled
/// - The override matcher fails to build
#[cfg(feature = "ignore")]
pub fn codeowners_entry_to_matcher(
    entry: &CodeownersEntry,
) -> Result<CodeownersEntryMatcher, PatternError> {
    let codeowners_dir = entry.source_file.parent().ok_or_else(|| PatternError {
        pattern: entry.pattern.clone(),
        source_file: entry.source_file.clone(),
        message: "CODEOWNERS entry has no parent directory".to_string(),
    })?;

    let mut builder = ignore::overrides::OverrideBuilder::new(codeowners_dir);

    // Transform directory patterns to match GitHub CODEOWNERS behavior
    let pattern = normalize_codeowners_pattern(&entry.pattern);

    builder.add(&pattern).map_err(|e| PatternError {
        pattern: entry.pattern.clone(),
        source_file: entry.source_file.clone(),
        message: format!(
            "Invalid pattern '{}' (normalized from '{}'): {}",
            pattern, entry.pattern, e
        ),
    })?;

    let override_matcher = builder.build().map_err(|e| PatternError {
        pattern: entry.pattern.clone(),
        source_file: entry.source_file.clone(),
        message: format!("Failed to build override matcher: {}", e),
    })?;

    Ok(CodeownersEntryMatcher {
        source_file: entry.source_file.clone(),
        line_number: entry.line_number,
        pattern: entry.pattern.clone(),
        owners: entry.owners.clone(),
        tags: entry.tags.clone(),
        override_matcher,
    })
}

/// Represents an owner of code files.
///
/// Owners can be GitHub users, teams, email addresses, or special markers
/// indicating unowned files.
///
/// # Examples
///
/// * `@username` - A GitHub user (OwnerType::User)
/// * `@org/team-name` - A GitHub team (OwnerType::Team)
/// * `user@example.com` - An email address (OwnerType::Email)
/// * `NOOWNER` - Explicitly unowned (OwnerType::Unowned)
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Owner {
    /// The raw identifier string (e.g., "@rust-team", "dev@example.com")
    pub identifier: String,
    /// The classified type of this owner
    pub owner_type: OwnerType,
}

/// Classification of owner types in CODEOWNERS files.
///
/// Used to distinguish between different forms of ownership identifiers.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum OwnerType {
    /// A GitHub user (e.g., `@username`)
    User,
    /// A GitHub team (e.g., `@org/team-name`)
    Team,
    /// An email address (e.g., `user@example.com`)
    Email,
    /// Explicitly marked as unowned (NOOWNER keyword)
    Unowned,
    /// Could not determine the owner type
    Unknown,
}

impl std::fmt::Display for OwnerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnerType::User => write!(f, "User"),
            OwnerType::Team => write!(f, "Team"),
            OwnerType::Email => write!(f, "Email"),
            OwnerType::Unowned => write!(f, "Unowned"),
            OwnerType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// A metadata tag for categorizing code ownership rules.
///
/// Tags are optional annotations in CODEOWNERS entries prefixed with `#`.
/// They can be used to categorize files by domain, criticality, or other attributes.
///
/// # Examples
///
/// * `#backend` - Backend service code
/// * `#critical` - Security-critical files
/// * `#frontend` - Frontend components
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Tag(pub String);

/// Output format for CLI commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    /// Human-readable text output (default)
    Text,
    /// JSON format for programmatic consumption
    Json,
    /// Binary format for efficient serialization
    Bincode,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Bincode => write!(f, "bincode"),
        }
    }
}

// Cache related types

/// A file with its resolved ownership information.
///
/// This represents the final computed ownership for a specific file,
/// combining rules from CODEOWNERS files and inline declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Path to the file (relative to repository root)
    pub path: PathBuf,
    /// All owners assigned to this file
    pub owners: Vec<Owner>,
    /// All tags associated with this file
    pub tags: Vec<Tag>,
}

/// Pre-computed cache of CODEOWNERS information for fast lookups.
///
/// The cache stores parsed CODEOWNERS rules, file ownership mappings,
/// and reverse lookup indexes for efficient querying.
///
/// # Cache Invalidation
///
/// The cache is invalidated when the repository state changes, detected
/// via the `hash` field which combines:
/// - HEAD commit hash
/// - Index/staging area state
/// - Unstaged file changes (excluding cache files themselves)
#[derive(Debug)]
pub struct CodeownersCache {
    /// SHA-256 hash of the repository state for cache invalidation
    pub hash: [u8; 32],
    /// All parsed CODEOWNERS entries
    pub entries: Vec<CodeownersEntry>,
    /// All files with their resolved ownership
    pub files: Vec<FileEntry>,
    /// Reverse lookup: owner → list of owned files
    pub owners_map: std::collections::HashMap<Owner, Vec<PathBuf>>,
    /// Reverse lookup: tag → list of tagged files
    pub tags_map: std::collections::HashMap<Tag, Vec<PathBuf>>,
}

impl Serialize for CodeownersCache {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("CodeownersCache", 4)?;
        state.serialize_field("hash", &self.hash)?;
        state.serialize_field("entries", &self.entries)?;
        state.serialize_field("files", &self.files)?;

        // Convert owners_map to a serializable format
        let owners_map_serializable: Vec<(&Owner, &Vec<PathBuf>)> =
            self.owners_map.iter().collect();
        state.serialize_field("owners_map", &owners_map_serializable)?;

        // Convert tags_map to a serializable format
        let tags_map_serializable: Vec<(&Tag, &Vec<PathBuf>)> = self.tags_map.iter().collect();
        state.serialize_field("tags_map", &tags_map_serializable)?;

        state.end()
    }
}

impl<'de> Deserialize<'de> for CodeownersCache {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CodeownersCacheHelper {
            hash: [u8; 32],
            entries: Vec<CodeownersEntry>,
            files: Vec<FileEntry>,
            owners_map: Vec<(Owner, Vec<PathBuf>)>,
            tags_map: Vec<(Tag, Vec<PathBuf>)>,
        }

        let helper = CodeownersCacheHelper::deserialize(deserializer)?;

        // Convert back to HashMap
        let mut owners_map = std::collections::HashMap::new();
        for (owner, paths) in helper.owners_map {
            owners_map.insert(owner, paths);
        }

        let mut tags_map = std::collections::HashMap::new();
        for (tag, paths) in helper.tags_map {
            tags_map.insert(tag, paths);
        }

        Ok(CodeownersCache {
            hash: helper.hash,
            entries: helper.entries,
            files: helper.files,
            owners_map,
            tags_map,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEncoding {
    Bincode,
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_codeowners_pattern_directory_patterns() {
        // Directory patterns ending with / should be converted to /**
        assert_eq!(
            normalize_codeowners_pattern("/builtin/logical/aws/"),
            "/builtin/logical/aws/**"
        );
        assert_eq!(
            normalize_codeowners_pattern("src/components/"),
            "src/components/**"
        );
        assert_eq!(normalize_codeowners_pattern("docs/"), "docs/**");
        assert_eq!(normalize_codeowners_pattern("/"), "/**");
    }

    #[test]
    fn test_normalize_codeowners_pattern_already_globbed() {
        // Patterns already ending with */ or **/ should be left as-is
        assert_eq!(
            normalize_codeowners_pattern("/builtin/logical/aws/*"),
            "/builtin/logical/aws/*"
        );
        assert_eq!(
            normalize_codeowners_pattern("/builtin/logical/aws/**"),
            "/builtin/logical/aws/**"
        );
        assert_eq!(normalize_codeowners_pattern("src/*/"), "src/*/");
        assert_eq!(normalize_codeowners_pattern("docs/**/"), "docs/**/");
    }

    #[test]
    fn test_normalize_codeowners_pattern_file_patterns() {
        // File patterns should be left as-is
        assert_eq!(normalize_codeowners_pattern("*.rs"), "*.rs");
        assert_eq!(normalize_codeowners_pattern("/src/main.rs"), "/src/main.rs");
        assert_eq!(normalize_codeowners_pattern("package.json"), "package.json");
        assert_eq!(normalize_codeowners_pattern("**/*.ts"), "**/*.ts");
    }

    #[test]
    fn test_normalize_codeowners_pattern_edge_cases() {
        // Edge cases
        assert_eq!(normalize_codeowners_pattern(""), "");
        assert_eq!(normalize_codeowners_pattern("file"), "file");
        assert_eq!(normalize_codeowners_pattern("./relative/"), "./relative/**");
        assert_eq!(normalize_codeowners_pattern("../parent/"), "../parent/**");
    }

    #[test]
    fn test_normalize_codeowners_pattern_complex_patterns() {
        // Complex patterns that should and shouldn't be transformed
        assert_eq!(
            normalize_codeowners_pattern("/path/with spaces/"),
            "/path/with spaces/**"
        );
        assert_eq!(
            normalize_codeowners_pattern("/path-with-dashes/"),
            "/path-with-dashes/**"
        );
        assert_eq!(
            normalize_codeowners_pattern("/path.with.dots/"),
            "/path.with.dots/**"
        );

        // These should not be transformed
        assert_eq!(
            normalize_codeowners_pattern("/path/*/something"),
            "/path/*/something"
        );
        assert_eq!(
            normalize_codeowners_pattern("/path/**/something"),
            "/path/**/something"
        );
    }

    #[cfg(feature = "ignore")]
    #[test]
    fn test_codeowners_entry_to_matcher_directory_pattern_github_behavior() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory structure for testing
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create directory structure: builtin/logical/aws/
        let aws_dir = base_path.join("builtin").join("logical").join("aws");
        fs::create_dir_all(&aws_dir).unwrap();

        // Create some test files
        fs::write(aws_dir.join("config.go"), "package aws").unwrap();
        fs::write(aws_dir.join("client.go"), "package aws").unwrap();

        // Create subdirectory with files
        let sub_dir = aws_dir.join("auth");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(sub_dir.join("token.go"), "package auth").unwrap();

        // Create CODEOWNERS file
        let codeowners_path = base_path.join("CODEOWNERS");

        // Test directory pattern: /builtin/logical/aws/ should match all files under it
        let entry = CodeownersEntry {
            source_file: codeowners_path.clone(),
            line_number: 1,
            pattern: "/builtin/logical/aws/".to_string(),
            owners: vec![Owner {
                identifier: "@hashicorp/vault-ecosystem".to_string(),
                owner_type: OwnerType::Team,
            }],
            tags: vec![],
        };

        let matcher = codeowners_entry_to_matcher(&entry).unwrap();

        // Test files that should match
        let test_files = vec![
            "builtin/logical/aws/config.go",
            "builtin/logical/aws/client.go",
            "builtin/logical/aws/auth/token.go",
        ];

        for file_path in test_files {
            let path = base_path.join(file_path);
            let relative_path = path.strip_prefix(base_path).unwrap();

            // The override matcher should match these files
            let is_match = matcher
                .override_matcher
                .matched(relative_path, false)
                .is_whitelist();
            assert!(
                is_match,
                "Pattern '/builtin/logical/aws/' should match file '{}' (like GitHub)",
                relative_path.display()
            );
        }

        // Test files that should NOT match
        let non_matching_files = vec![
            "builtin/logical/database/config.go",
            "builtin/auth/aws/config.go",
            "other/aws/config.go",
        ];

        for file_path in non_matching_files {
            let path = base_path.join(file_path);
            // Create parent dirs for these test files
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, "test content").unwrap();

            let relative_path = path.strip_prefix(base_path).unwrap();
            let is_match = matcher
                .override_matcher
                .matched(relative_path, false)
                .is_whitelist();
            assert!(
                !is_match,
                "Pattern '/builtin/logical/aws/' should NOT match file '{}'",
                relative_path.display()
            );
        }
    }
}
