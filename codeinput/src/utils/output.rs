//! Output utilities for CLI progress and status messages

use std::io::Write;

use super::app_config::AppConfig;

/// Check if quiet mode is enabled
fn is_quiet() -> bool {
    AppConfig::fetch().map(|c| c.quiet).unwrap_or(false)
}

/// Print a message if not in quiet mode
pub fn print(msg: &str) {
    if !is_quiet() {
        print!("{}", msg);
        std::io::stdout().flush().ok();
    }
}

/// Print a line if not in quiet mode
pub fn println(msg: &str) {
    if !is_quiet() {
        println!("{}", msg);
    }
}
