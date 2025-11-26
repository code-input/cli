use std::path::PathBuf;

#[cfg(feature = "ignore")]
use ignore::overrides::Override;
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// Normalizes a CODEOWNERS pattern to match GitHub's behavior
///
/// GitHub CODEOWNERS directory matching rules:
/// - `/path/to/dir/` matches all files and subdirectories under that path (converted to `/path/to/dir/**`)
/// - `/path/to/dir/*` matches direct files only (kept as-is)
/// - `/path/to/dir/**` matches everything recursively (kept as-is)
/// - Other patterns are kept as-is
#[cfg(any(feature = "ignore", test))]
fn normalize_codeowners_pattern(pattern: &str) -> String {
    // If pattern ends with `/` but not `*/` or `**/`, convert to `/**`
    if pattern.ends_with('/') && !pattern.ends_with("*/") && !pattern.ends_with("**/") {
        format!("{}**", pattern)
    } else {
        pattern.to_string()
    }
}

/// CODEOWNERS entry with source tracking
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CodeownersEntry {
    pub source_file: PathBuf,
    pub line_number: usize,
    pub pattern: String,
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
}

/// Inline CODEOWNERS entry for file-specific ownership
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

#[cfg(feature = "ignore")]
pub fn codeowners_entry_to_matcher(entry: &CodeownersEntry) -> CodeownersEntryMatcher {
    let codeowners_dir = match entry.source_file.parent() {
        Some(dir) => dir,
        None => {
            eprintln!(
                "CODEOWNERS entry has no parent directory: {}",
                entry.source_file.display()
            );
            panic!("Invalid CODEOWNERS entry without parent directory");
        }
    };

    let mut builder = ignore::overrides::OverrideBuilder::new(codeowners_dir);

    // Transform directory patterns to match GitHub CODEOWNERS behavior
    let pattern = normalize_codeowners_pattern(&entry.pattern);

    if let Err(e) = builder.add(&pattern) {
        eprintln!(
            "Invalid pattern '{}' (normalized from '{}') in {}: {}",
            pattern,
            entry.pattern,
            entry.source_file.display(),
            e
        );
        panic!("Invalid CODEOWNERS entry pattern");
    }
    let override_matcher: Override = match builder.build() {
        Ok(o) => o,
        Err(e) => {
            eprintln!(
                "Failed to build override for pattern '{}': {}",
                entry.pattern, e
            );
            panic!("Failed to build CODEOWNERS entry matcher");
        }
    };

    CodeownersEntryMatcher {
        source_file: entry.source_file.clone(),
        line_number: entry.line_number,
        pattern: entry.pattern.clone(),
        owners: entry.owners.clone(),
        tags: entry.tags.clone(),
        override_matcher,
    }
}

/// Detailed owner representation
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Owner {
    pub identifier: String,
    pub owner_type: OwnerType,
}

/// Owner type classification
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum OwnerType {
    User,
    Team,
    Email,
    Unowned,
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

/// Tag representation
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Tag(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
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
/// File entry in the ownership cache
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct FileEntry {
    pub path: PathBuf,
    pub owners: Vec<Owner>,
    pub tags: Vec<Tag>,
}

/// Cache for storing parsed CODEOWNERS information
#[derive(Debug)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CodeownersCache {
    pub hash: [u8; 32],
    pub entries: Vec<CodeownersEntry>,
    pub files: Vec<FileEntry>,
    // Derived data for lookups
    pub owners_map: std::collections::HashMap<Owner, Vec<PathBuf>>,
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

        let matcher = codeowners_entry_to_matcher(&entry);

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
