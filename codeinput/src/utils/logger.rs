use slog::o;
use slog::Drain;
#[cfg(all(target_os = "linux", feature = "journald"))]
use slog_journald::JournaldDrain;
#[cfg(feature = "syslog")]
use slog_syslog::Facility;

use super::app_config::AppConfig;
use super::error::Result;
use super::types::LogLevel;

pub fn setup_logging() -> Result<slog_scope::GlobalLoggerGuard> {
    // Setup Logging
    let guard = slog_scope::set_global_logger(default_root_logger()?);
    slog_stdlog::init()?;

    // Set log level for the log crate (used by ignore and other crates)
    let config = AppConfig::fetch().unwrap_or(AppConfig {
        debug: false,
        log_level: LogLevel::Info,
        cache_file: ".codeowners.cache".to_string(),
        quiet: false,
    });

    let log_level = match config.log_level {
        LogLevel::Debug => log::LevelFilter::Debug,
        LogLevel::Info => log::LevelFilter::Info,
        LogLevel::Warn => log::LevelFilter::Warn,
        LogLevel::Error => log::LevelFilter::Error,
    };
    
    log::set_max_level(log_level);

    Ok(guard)
}

pub fn default_root_logger() -> Result<slog::Logger> {
    // Get configured log level
    let config = AppConfig::fetch().unwrap_or(AppConfig {
        debug: false,
        log_level: LogLevel::Info,
        cache_file: ".codeowners.cache".to_string(),
        quiet: false,
    });

    let slog_level = match config.log_level {
        LogLevel::Debug => slog::Level::Debug,
        LogLevel::Info => slog::Level::Info,
        LogLevel::Warn => slog::Level::Warning,
        LogLevel::Error => slog::Level::Error,
    };

    // Create drains
    let drain = slog::Duplicate(default_discard()?, default_discard()?).fuse();

    // Merge drains with level filtering
    #[cfg(feature = "termlog")]
    let drain = slog::Duplicate(
        slog::LevelFilter::new(default_term_drain().unwrap_or(default_discard()?), slog_level).fuse(),
        drain
    ).fuse();
    #[cfg(feature = "syslog")]
    let drain = slog::Duplicate(
        slog::LevelFilter::new(default_syslog_drain().unwrap_or(default_discard()?), slog_level).fuse(),
        drain
    ).fuse();
    #[cfg(feature = "journald")]
    #[cfg(target_os = "linux")]
    let drain = slog::Duplicate(
        slog::LevelFilter::new(default_journald_drain().unwrap_or(default_discard()?), slog_level).fuse(),
        drain,
    )
    .fuse();

    // Create Logger
    let logger = slog::Logger::root(drain, o!("who" => "codeinput"));

    // Return Logger
    Ok(logger)
}

fn default_discard() -> Result<slog_async::Async> {
    let drain = slog_async::Async::default(slog::Discard);

    Ok(drain)
}

// term drain: Log to Terminal
#[cfg(feature = "termlog")]
fn default_term_drain() -> Result<slog_async::Async> {
    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let term = slog_term::FullFormat::new(plain);

    let drain = slog_async::Async::default(term.build().fuse());

    Ok(drain)
}

// syslog drain: Log to syslog
#[cfg(feature = "syslog")]
fn default_syslog_drain() -> Result<slog_async::Async> {
    let syslog = slog_syslog::unix_3164(Facility::LOG_USER)?;

    let drain = slog_async::Async::default(syslog.fuse());

    Ok(drain)
}

#[cfg(all(target_os = "linux", feature = "journald"))]
fn default_journald_drain() -> Result<slog_async::Async> {
    let journald = JournaldDrain.ignore_res();
    let drain = slog_async::Async::default(journald);

    Ok(drain)
}
