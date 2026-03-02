//! Output utilities for CLI progress and status messages
//!
//! All output goes to stderr to keep stdout clean for data/LSP protocol.

use std::io::{stderr, Write};

use super::app_config::AppConfig;

/// Check if quiet mode is enabled
fn is_quiet() -> bool {
    AppConfig::fetch().map(|c| c.quiet).unwrap_or(false)
}

/// Print a message to stderr if not in quiet mode
pub fn print(msg: &str) {
    if !is_quiet() {
        let mut stderr = stderr();
        write!(stderr, "{}", msg).ok();
        stderr.flush().ok();
    }
}

/// Print a line to stderr if not in quiet mode
pub fn println(msg: &str) {
    if !is_quiet() {
        writeln!(stderr(), "{}", msg).ok();
    }
}
