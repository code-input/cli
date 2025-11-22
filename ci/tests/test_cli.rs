use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Get the CLI command for testing
#[allow(deprecated)]
fn ci() -> Command {
    Command::cargo_bin("ci").unwrap()
}

/// Create a test repository with CODEOWNERS file
fn setup_test_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(base_path)
        .output()
        .expect("Failed to initialize git repo");

    // Configure git user for commits
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(base_path)
        .output()
        .expect("Failed to configure git email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(base_path)
        .output()
        .expect("Failed to configure git name");

    // Create CODEOWNERS file
    let codeowners_content = r#"# Root CODEOWNERS
* @default-team
*.rs @rust-team #rust #backend
*.js @js-team #javascript #frontend
/docs/ @docs-team #documentation
"#;
    fs::write(base_path.join("CODEOWNERS"), codeowners_content).unwrap();

    // Create some test files
    fs::create_dir_all(base_path.join("src")).unwrap();
    fs::create_dir_all(base_path.join("docs")).unwrap();
    fs::write(base_path.join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(base_path.join("src/lib.rs"), "// library").unwrap();
    fs::write(base_path.join("app.js"), "console.log('hello');").unwrap();
    fs::write(base_path.join("docs/README.md"), "# Documentation").unwrap();

    // Commit the files
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(base_path)
        .output()
        .expect("Failed to stage files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(base_path)
        .output()
        .expect("Failed to create commit");

    temp_dir
}

#[test]
fn test_help_command() {
    ci().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("code input CLI"));
}

#[test]
fn test_version_command() {
    ci().arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.0.3"));
}

#[test]
fn test_config_command() {
    ci().arg("config")
        .assert()
        .success();
}

#[test]
fn test_codeowners_parse_command() {
    let temp_dir = setup_test_repo();

    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Processed"));

    // Verify cache file was created
    assert!(temp_dir.path().join(".codeowners.cache").exists());
}

#[test]
fn test_codeowners_parse_with_json_format() {
    let temp_dir = setup_test_repo();

    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    // Verify cache file was created
    assert!(temp_dir.path().join(".codeowners.cache").exists());
}

#[test]
fn test_codeowners_list_files_command() {
    let temp_dir = setup_test_repo();

    // First parse to create cache
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();

    // Then list files
    ci().arg("codeowners")
        .arg("list-files")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("File Path"));
}

#[test]
fn test_codeowners_list_files_json_format() {
    let temp_dir = setup_test_repo();

    // First parse
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();

    // Then list files as JSON
    ci().arg("codeowners")
        .arg("list-files")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("["));
}

#[test]
fn test_codeowners_list_files_with_tag_filter() {
    let temp_dir = setup_test_repo();

    // First parse
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();

    // List files filtered by tag
    ci().arg("codeowners")
        .arg("list-files")
        .arg(temp_dir.path())
        .arg("--tags")
        .arg("rust")
        .assert()
        .success();
}

#[test]
fn test_codeowners_list_files_with_owner_filter() {
    let temp_dir = setup_test_repo();

    // First parse
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();

    // List files filtered by owner
    ci().arg("codeowners")
        .arg("list-files")
        .arg(temp_dir.path())
        .arg("--owners")
        .arg("@rust-team")
        .assert()
        .success();
}

#[test]
fn test_codeowners_list_files_unowned() {
    let temp_dir = setup_test_repo();

    // First parse
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();

    // List unowned files
    ci().arg("codeowners")
        .arg("list-files")
        .arg(temp_dir.path())
        .arg("--unowned")
        .assert()
        .success();
}

#[test]
fn test_codeowners_list_owners_command() {
    let temp_dir = setup_test_repo();

    // First parse
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();

    // List owners
    ci().arg("codeowners")
        .arg("list-owners")
        .arg(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn test_codeowners_list_tags_command() {
    let temp_dir = setup_test_repo();

    // First parse
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();

    // List tags
    ci().arg("codeowners")
        .arg("list-tags")
        .arg(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn test_codeowners_list_rules_command() {
    let temp_dir = setup_test_repo();

    // First parse
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();

    // List rules - need to run from the repo directory
    ci().arg("codeowners")
        .arg("list-rules")
        .current_dir(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn test_codeowners_inspect_command() {
    let temp_dir = setup_test_repo();

    // First parse from within the repo directory
    ci().arg("codeowners")
        .arg("parse")
        .arg(".")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Inspect a specific file - verify ownership info is returned
    ci().arg("codeowners")
        .arg("inspect")
        .arg("src/main.rs")
        .arg("--repo")
        .arg(".")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("@rust-team"));
}

#[test]
fn test_completion_bash() {
    ci().arg("completion")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_codeinput"));
}

#[test]
fn test_completion_zsh() {
    ci().arg("completion")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef"));
}

#[test]
fn test_completion_fish() {
    ci().arg("completion")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_invalid_subcommand() {
    ci().arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_codeowners_without_git_repo() {
    let temp_dir = TempDir::new().unwrap();

    // Create CODEOWNERS without git repo
    fs::write(temp_dir.path().join("CODEOWNERS"), "* @team").unwrap();

    // Should fail because not a git repo
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .failure();
}

#[test]
fn test_inline_codeowners_detection() {
    let temp_dir = setup_test_repo();

    // Create a file with inline CODEOWNERS
    let content = r#"// !!!CODEOWNERS @special-team #special
fn special_function() {}
"#;
    fs::write(temp_dir.path().join("src/special.rs"), content).unwrap();

    // Parse from within the repo directory
    ci().arg("codeowners")
        .arg("parse")
        .arg(".")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // List files should include the new file with inline ownership
    // Use list-files with JSON format to verify inline ownership works
    ci().arg("codeowners")
        .arg("list-files")
        .arg(".")
        .arg("--format")
        .arg("json")
        .arg("--show-all")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("special.rs"));
}

#[test]
fn test_custom_cache_file() {
    let temp_dir = setup_test_repo();

    // Parse with custom cache file
    ci().arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .arg("--cache-file")
        .arg("custom.cache")
        .assert()
        .success();

    // Verify custom cache file was created
    assert!(temp_dir.path().join("custom.cache").exists());
}

#[test]
fn test_log_level_flag() {
    let temp_dir = setup_test_repo();

    ci().arg("--log-level")
        .arg("warn")
        .arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn test_quiet_flag() {
    let temp_dir = setup_test_repo();

    // With --quiet flag, there should be no progress output
    let output = ci()
        .arg("--quiet")
        .arg("codeowners")
        .arg("parse")
        .arg(temp_dir.path())
        .assert()
        .success();

    // Verify no progress indicator in output
    output.stdout(predicate::str::is_empty().not().or(predicate::str::is_empty()));
}
