use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};
use std::path::PathBuf;

use codeinput::core::{
    commands::{
        self,
        infer_owners::{InferAlgorithm, InferScope},
    },
    types::{CacheEncoding, OutputFormat},
};
use codeinput::utils::app_config::AppConfig;
use codeinput::utils::error::Result;
use codeinput::utils::types::LogLevel;

#[derive(Parser, Debug)]
#[command(
    name = "codeinput",
    author,
    about,
    long_about = "code input CLI",
    version
)]
//TODO: #[clap(setting = AppSettings::SubcommandRequired)]
//TODO: #[clap(global_setting(AppSettings::DeriveDisplayOrder))]
pub struct Cli {
    /// Set a custom config file
    /// TODO: parse(from_os_str)
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Set a custom config file
    #[arg(name = "debug", short, long = "debug", value_name = "DEBUG")]
    pub debug: Option<bool>,

    /// Set Log Level
    #[arg(
        name = "log_level",
        short,
        long = "log-level",
        value_name = "LOG_LEVEL"
    )]
    pub log_level: Option<LogLevel>,

    /// Subcommands
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[clap(
        name = "codeowners",
        about = "Manage and analyze CODEOWNERS files",
        long_about = "Tools for parsing, validating and querying CODEOWNERS files"
    )]
    Codeowners {
        #[clap(subcommand)]
        subcommand: CodeownersSubcommand,
    },
    #[clap(
        name = "completion",
        about = "Generate completion scripts",
        long_about = None,
        )]
    Completion {
        #[clap(subcommand)]
        subcommand: CompletionSubcommand,
    },
    #[clap(
        name = "config",
        about = "Show Configuration",
        long_about = None,
    )]
    Config,
}

#[derive(Subcommand, PartialEq, Debug)]
enum CompletionSubcommand {
    #[clap(about = "generate the autocompletion script for bash")]
    Bash,
    #[clap(about = "generate the autocompletion script for zsh")]
    Zsh,
    #[clap(about = "generate the autocompletion script for fish")]
    Fish,
}

#[derive(Subcommand, PartialEq, Debug)]
pub(crate) enum CodeownersSubcommand {
    #[clap(
        name = "parse",
        about = "Preprocess CODEOWNERS files and build ownership map"
    )]
    Parse {
        /// Directory path to analyze (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Custom cache file location
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,

        /// Output format: json|bincode
        #[arg(long, value_name = "FORMAT", default_value = "bincode", value_parser = parse_cache_encoding)]
        format: CacheEncoding,
    },

    #[clap(
        name = "list-files",
        about = "Find and list files with their owners based on filter criteria"
    )]
    ListFiles {
        /// Directory path to analyze (default: current directory)
        #[arg(default_value = ".")]
        path: Option<PathBuf>,

        /// Only show files with specified tags
        #[arg(long, value_name = "LIST")]
        tags: Option<String>,

        /// Only show files owned by these owners
        #[arg(long, value_name = "LIST")]
        owners: Option<String>,

        /// Show only unowned files
        #[arg(long)]
        unowned: bool,

        /// Show all files including unowned/untagged
        #[arg(long)]
        show_all: bool,

        /// Output format: text|json|bincode
        #[arg(long, value_name = "FORMAT", default_value = "text", value_parser = parse_output_format)]
        format: OutputFormat,

        /// Custom cache file location
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,
    },

    #[clap(
        name = "list-owners",
        about = "Display aggregated owner statistics and associations"
    )]
    ListOwners {
        /// Directory path to analyze (default: current directory)
        #[arg(default_value = ".")]
        path: Option<PathBuf>,

        /// Output format: text|json|bincode
        #[arg(long, value_name = "FORMAT", default_value = "text", value_parser = parse_output_format)]
        format: OutputFormat,

        /// Custom cache file location
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,
    },
    #[clap(
        name = "list-tags",
        about = "Audit and analyze tag usage across CODEOWNERS files"
    )]
    ListTags {
        /// Directory path to analyze (default: current directory)
        #[arg(default_value = ".")]
        path: Option<PathBuf>,

        /// Output format: text|json|bincode
        #[arg(long, value_name = "FORMAT", default_value = "text", value_parser = parse_output_format)]
        format: OutputFormat,

        /// Custom cache file location
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,
    },
    #[clap(
        name = "list-rules",
        about = "Display all CODEOWNERS rules from the cache"
    )]
    ListRules {
        /// Output format: text|json|bincode
        #[arg(long, value_name = "FORMAT", default_value = "text", value_parser = parse_output_format)]
        format: OutputFormat,

        /// Custom cache file location
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,
    },
    #[clap(
        name = "inspect",
        about = "Inspect ownership and tags for a specific file"
    )]
    Inspect {
        /// File path to inspect
        #[arg(value_name = "FILE")]
        file_path: PathBuf,

        /// Directory path to analyze (default: current directory)
        #[arg(short, long, default_value = ".")]
        repo: Option<PathBuf>,

        /// Output format: text|json|bincode
        #[arg(long, value_name = "FORMAT", default_value = "text", value_parser = parse_output_format)]
        format: OutputFormat,

        /// Custom cache file location
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,
    },
    #[clap(
        name = "infer-owners",
        about = "Infer file ownership from git history and blame information"
    )]
    InferOwners {
        /// Directory path to analyze (default: current directory)
        #[arg(default_value = ".")]
        path: Option<PathBuf>,

        /// Scope of analysis: all files or only unowned files
        #[arg(long, value_name = "SCOPE", default_value = "unowned", value_parser = parse_infer_scope)]
        scope: InferScope,

        /// Algorithm for ownership determination
        #[arg(long, value_name = "ALGORITHM", default_value = "lines", value_parser = parse_infer_algorithm)]
        algorithm: InferAlgorithm,

        /// Only consider commits from last N days
        #[arg(long, value_name = "DAYS", default_value = "365")]
        lookback_days: u32,

        /// Minimum commits required to be considered owner
        #[arg(long, value_name = "COUNT", default_value = "3")]
        min_commits: u32,

        /// Minimum percentage of lines/commits to be considered owner
        #[arg(long, value_name = "PERCENT", default_value = "20")]
        min_percentage: u32,

        /// Custom cache file location
        #[arg(long, value_name = "FILE", default_value = ".codeowners.cache")]
        cache_file: Option<PathBuf>,

        /// Output file to write CODEOWNERS entries
        #[arg(long, short = 'o', value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

pub fn cli_match() -> Result<()> {
    // Parse the command line arguments
    let cli = Cli::parse();

    // Merge clap config file if the value is set
    AppConfig::merge_config(cli.config.as_deref())?;

    let app = Cli::command();
    let matches = app.get_matches();

    AppConfig::merge_args(matches)?;

    // Execute the subcommand
    match &cli.command {
        Commands::Codeowners { subcommand } => codeowners(subcommand)?,
        Commands::Completion { subcommand } => {
            let mut app = Cli::command();
            match subcommand {
                CompletionSubcommand::Bash => {
                    generate(Bash, &mut app, "codeinput", &mut std::io::stdout());
                }
                CompletionSubcommand::Zsh => {
                    generate(Zsh, &mut app, "codeinput", &mut std::io::stdout());
                }
                CompletionSubcommand::Fish => {
                    generate(Fish, &mut app, "codeinput", &mut std::io::stdout());
                }
            }
        }
        Commands::Config => commands::config::run()?,
    }

    Ok(())
}

/// Handle codeowners subcommands
pub(crate) fn codeowners(subcommand: &CodeownersSubcommand) -> Result<()> {
    match subcommand {
        CodeownersSubcommand::Parse {
            path,
            cache_file,
            format,
        } => commands::parse::run(path, cache_file.as_deref(), *format),
        CodeownersSubcommand::ListFiles {
            path,
            tags,
            owners,
            unowned,
            show_all,
            format,
            cache_file,
        } => commands::list_files::run(
            path.as_deref(),
            tags.as_deref(),
            owners.as_deref(),
            *unowned,
            *show_all,
            format,
            cache_file.as_deref(),
        ),
        CodeownersSubcommand::ListOwners {
            path,
            format,
            cache_file,
        } => commands::list_owners::run(path.as_deref(), format, cache_file.as_deref()),
        CodeownersSubcommand::ListTags {
            path,
            format,
            cache_file,
        } => commands::list_tags::run(path.as_deref(), format, cache_file.as_deref()),
        CodeownersSubcommand::ListRules { format, cache_file } => {
            commands::list_rules::run(format, cache_file.as_deref())
        }
        CodeownersSubcommand::Inspect {
            file_path,
            repo,
            format,
            cache_file,
        } => commands::inspect::run(file_path, repo.as_deref(), format, cache_file.as_deref()),
        CodeownersSubcommand::InferOwners {
            path,
            scope,
            algorithm,
            lookback_days,
            min_commits,
            min_percentage,
            cache_file,
            output,
        } => commands::infer_owners::run(
            path.as_deref(),
            scope,
            algorithm,
            *lookback_days,
            *min_commits,
            *min_percentage,
            cache_file.as_deref(),
            output.as_deref(),
        ),
    }
}

fn parse_output_format(s: &str) -> std::result::Result<OutputFormat, String> {
    match s.to_lowercase().as_str() {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        "bincode" => Ok(OutputFormat::Bincode),
        _ => Err(format!("Invalid output format: {}", s)),
    }
}

fn parse_cache_encoding(s: &str) -> std::result::Result<CacheEncoding, String> {
    match s.to_lowercase().as_str() {
        "bincode" => Ok(CacheEncoding::Bincode),
        "json" => Ok(CacheEncoding::Json),
        _ => Err(format!("Invalid cache encoding: {}", s)),
    }
}

fn parse_infer_scope(s: &str) -> std::result::Result<InferScope, String> {
    match s.to_lowercase().as_str() {
        "all" => Ok(InferScope::All),
        "unowned" => Ok(InferScope::Unowned),
        _ => Err(format!("Invalid scope: {}. Valid options: all, unowned", s)),
    }
}

fn parse_infer_algorithm(s: &str) -> std::result::Result<InferAlgorithm, String> {
    match s.to_lowercase().as_str() {
        "commits" => Ok(InferAlgorithm::Commits),
        "lines" => Ok(InferAlgorithm::Lines),
        "recent" => Ok(InferAlgorithm::Recent),
        _ => Err(format!(
            "Invalid algorithm: {}. Valid options: commits, lines, recent",
            s
        )),
    }
}
