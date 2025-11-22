use crate::utils::error::Result;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::parser::parse_owner;
use super::types::{InlineCodeownersEntry, Owner, Tag};

/// Detects inline CODEOWNERS declaration in the first 50 lines of a file
pub fn detect_inline_codeowners(file_path: &Path) -> Result<Option<InlineCodeownersEntry>> {
    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return Ok(None), // File doesn't exist or can't be read
    };

    let reader = BufReader::new(file);
    let lines = reader.lines().take(50);

    for (line_num, line_result) in lines.enumerate() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue, // Skip lines that can't be read
        };

        if let Some(entry) = parse_inline_codeowners_line(&line, line_num + 1, file_path)? {
            return Ok(Some(entry));
        }
    }

    Ok(None)
}

/// Parse a single line for inline CODEOWNERS declaration
fn parse_inline_codeowners_line(
    line: &str, line_number: usize, file_path: &Path,
) -> Result<Option<InlineCodeownersEntry>> {
    // Look for !!!CODEOWNERS marker
    if let Some(marker_pos) = line.find("!!!CODEOWNERS") {
        // Extract everything after the marker
        let after_marker = &line[marker_pos + "!!!CODEOWNERS".len()..];

        // Split by whitespace to get tokens
        let tokens: Vec<&str> = after_marker.split_whitespace().collect();

        if tokens.is_empty() {
            return Ok(None);
        }

        let mut owners: Vec<Owner> = Vec::new();
        let mut tags: Vec<Tag> = Vec::new();
        let mut i = 0;

        // Collect owners until a token starts with '#'
        while i < tokens.len() && !tokens[i].starts_with('#') {
            owners.push(parse_owner(tokens[i])?);
            i += 1;
        }

        // Collect tags
        while i < tokens.len() {
            let token = tokens[i];
            if let Some(tag_part) = token.strip_prefix('#') {
                if token == "#" {
                    // Standalone # means comment starts, break
                    break;
                } else {
                    // Extract tag name, but check if this might be a comment
                    // If the tag part is empty, it's probably a comment marker
                    if tag_part.is_empty() {
                        break;
                    }

                    // Special handling for common comment patterns
                    // If the next token looks like end of comment (like "-->"), still treat as tag
                    let next_token = if i + 1 < tokens.len() {
                        Some(tokens[i + 1])
                    } else {
                        None
                    };

                    match next_token {
                        Some("-->") | Some("*/") => {
                            // This is likely the end of a comment block, so the tag is valid
                            tags.push(Tag(tag_part.to_string()));
                            #[allow(unused_assignments)]
                            {
                                i += 1; // Necessary for loop correctness, even though we break immediately
                            }
                            break; // Stop after this tag since we hit comment end
                        }
                        Some(next) if next.starts_with('#') => {
                            // Next token is also a tag, so this is definitely a tag
                            tags.push(Tag(tag_part.to_string()));
                            i += 1;
                        }
                        Some(_) => {
                            // Next token doesn't start with # and isn't a comment ender
                            // This could be a comment, but we'll be conservative and treat as tag
                            // if it looks like a valid tag name (alphanumeric + common chars)
                            if tag_part
                                .chars()
                                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                            {
                                tags.push(Tag(tag_part.to_string()));
                                #[allow(unused_assignments)]
                                {
                                    i += 1; // Necessary for loop correctness, even though we break immediately
                                }
                                break; // Stop here as next token is likely a comment
                            } else {
                                break; // This is probably a comment
                            }
                        }
                        None => {
                            // This is the last token, treat as tag
                            tags.push(Tag(tag_part.to_string()));
                            i += 1;
                        }
                    }
                }
            } else {
                // Non-# token, this is part of a comment
                break;
            }
        }

        // Only return an entry if we have at least one owner
        if !owners.is_empty() {
            return Ok(Some(InlineCodeownersEntry {
                file_path: file_path.to_path_buf(),
                line_number,
                owners,
                tags,
            }));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_inline_codeowners_rust_comment() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let content = r#"// This is a Rust file
// !!!CODEOWNERS @user1 @org/team2 #tag1 #tag2
fn main() {
    println!("Hello world");
}
"#;
        fs::write(&file_path, content).unwrap();

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.file_path, file_path);
        assert_eq!(entry.line_number, 2);
        assert_eq!(entry.owners.len(), 2);
        assert_eq!(entry.owners[0].identifier, "@user1");
        assert_eq!(entry.owners[1].identifier, "@org/team2");
        assert_eq!(entry.tags.len(), 2);
        assert_eq!(entry.tags[0].0, "tag1");
        assert_eq!(entry.tags[1].0, "tag2");

        Ok(())
    }

    #[test]
    fn test_detect_inline_codeowners_javascript_comment() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.js");

        let content = r#"/* 
 * !!!CODEOWNERS @frontend-team #javascript
 */
function hello() {
    console.log("Hello");
}
"#;
        fs::write(&file_path, content).unwrap();

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.owners.len(), 1);
        assert_eq!(entry.owners[0].identifier, "@frontend-team");
        assert_eq!(entry.tags.len(), 1);
        assert_eq!(entry.tags[0].0, "javascript");

        Ok(())
    }

    #[test]
    fn test_detect_inline_codeowners_python_comment() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.py");

        let content = r#"#!/usr/bin/env python3
# !!!CODEOWNERS @python-team @user1 #backend #critical
"""
This is a Python module
"""

def main():
    pass
"#;
        fs::write(&file_path, content).unwrap();

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.line_number, 2);
        assert_eq!(entry.owners.len(), 2);
        assert_eq!(entry.owners[0].identifier, "@python-team");
        assert_eq!(entry.owners[1].identifier, "@user1");
        assert_eq!(entry.tags.len(), 2);
        assert_eq!(entry.tags[0].0, "backend");
        assert_eq!(entry.tags[1].0, "critical");

        Ok(())
    }

    #[test]
    fn test_detect_inline_codeowners_html_comment() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.html");

        let content = r#"<!DOCTYPE html>
<html>
<!-- !!!CODEOWNERS @web-team #frontend -->
<head>
    <title>Test</title>
</head>
</html>
"#;
        fs::write(&file_path, content).unwrap();

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.owners.len(), 1);
        assert_eq!(entry.owners[0].identifier, "@web-team");
        assert_eq!(entry.tags.len(), 1);
        assert_eq!(entry.tags[0].0, "frontend");

        Ok(())
    }

    #[test]
    fn test_detect_inline_codeowners_no_marker() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let content = r#"// This is a regular file
fn main() {
    println!("No CODEOWNERS marker here");
}
"#;
        fs::write(&file_path, content).unwrap();

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_detect_inline_codeowners_no_owners() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let content = r#"// !!!CODEOWNERS #just-tags
fn main() {
    println!("Only tags, no owners");
}
"#;
        fs::write(&file_path, content).unwrap();

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_detect_inline_codeowners_first_occurrence_only() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let content = r#"// !!!CODEOWNERS @first-owner #first-tag
fn main() {
    // !!!CODEOWNERS @second-owner #second-tag
    println!("Should only detect first occurrence");
}
"#;
        fs::write(&file_path, content).unwrap();

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.line_number, 1);
        assert_eq!(entry.owners[0].identifier, "@first-owner");
        assert_eq!(entry.tags[0].0, "first-tag");

        Ok(())
    }

    #[test]
    fn test_detect_inline_codeowners_beyond_50_lines() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let mut content = String::new();
        // Add 51 lines, with the marker on line 51
        for i in 1..=50 {
            content.push_str(&format!("// Line {}\n", i));
        }
        content.push_str("// !!!CODEOWNERS @should-not-be-found #beyond-limit\n");
        content.push_str("fn main() {}\n");

        fs::write(&file_path, content).unwrap();

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn test_detect_inline_codeowners_with_comment_after() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let content = r#"// !!!CODEOWNERS @user1 #tag1 # this is a comment after
fn main() {}
"#;
        fs::write(&file_path, content).unwrap();

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.owners.len(), 1);
        assert_eq!(entry.owners[0].identifier, "@user1");
        assert_eq!(entry.tags.len(), 1);
        assert_eq!(entry.tags[0].0, "tag1");

        Ok(())
    }

    #[test]
    fn test_detect_inline_codeowners_nonexistent_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.rs");

        let result = detect_inline_codeowners(&file_path)?;
        assert!(result.is_none());

        Ok(())
    }
}
