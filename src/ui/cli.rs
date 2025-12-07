//! CLI interface using clap

use clap::{Parser, Subcommand, ValueEnum};

/// CleanMyMac-rs - A cross-platform system cleaner
///
/// A powerful tool for cleaning caches, temporary files, and development artifacts
/// on macOS and Linux systems. Built with Rust for performance and safety.
///
/// Tip: Run 'cleanmymac-rs <COMMAND> --help' for detailed usage options.
#[derive(Parser, Debug)]
#[command(name = "cleanmymac-rs")]
#[command(author, version = env!("GIT_VERSION"), about)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Subcommand to run
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Path to configuration file
    #[arg(short, long, global = true)]
    pub config: Option<String>,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scan for cleanable files and display results
    ///
    /// Scans your system for caches, temporary files, and development artifacts
    /// that can be safely cleaned. Shows a detailed summary of what was found.
    #[command(visible_alias = "s")]
    Scan {
        /// Categories to scan (comma-separated)
        ///
        /// Available categories: system, brew, xcode, nodejs, python, rust, go, java, docker, android, heuristic, macapps, linuxpackages
        #[arg(short = 'C', long, value_delimiter = ',')]
        categories: Option<Vec<String>>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,

        /// Minimum size threshold (e.g., "100MB", "1GB")
        #[arg(long)]
        min_size: Option<String>,
    },

    /// Clean scanned files
    ///
    /// Removes the files found during scanning. By default, files are moved
    /// to the system trash for safety.
    #[command(visible_alias = "c")]
    Clean {
        /// Categories to clean (comma-separated)
        #[arg(short = 'C', long, value_delimiter = ',')]
        categories: Option<Vec<String>>,

        /// Perform a dry run (show what would be deleted)
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Skip confirmation prompts
        #[arg(short = 'y', long)]
        yes: bool,

        /// Permanently delete instead of moving to trash
        #[arg(long)]
        permanent: bool,

        /// Interactive mode (select items to clean)
        #[arg(short = 'i', long)]
        interactive: bool,

        /// Don't show progress bar
        #[arg(long)]
        quiet: bool,
    },

    /// Analyze storage usage
    ///
    /// Provides detailed analysis of disk usage, including largest files
    /// and size breakdown by file type.
    #[command(visible_alias = "a")]
    Analyze {
        /// Directory to analyze (defaults to home directory)
        #[arg(short, long)]
        path: Option<String>,

        /// Maximum depth to scan
        #[arg(short, long, default_value = "3")]
        depth: usize,

        /// Number of largest files to show
        #[arg(short, long, default_value = "10")]
        top: usize,
    },

    /// List available cleanup rules
    ///
    /// Shows all available cleanup rules with their categories and risk levels.
    #[command(visible_alias = "l")]
    List {
        /// Filter by category
        #[arg(short = 'C', long)]
        category: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Initialize or show configuration
    ///
    /// Creates a default configuration file or displays current settings.
    Config {
        /// Initialize default configuration
        #[arg(long)]
        init: bool,

        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Path for configuration file
        #[arg(long)]
        path: Option<String>,
    },

    /// Launch interactive TUI mode
    ///
    /// Opens a modern terminal user interface for interactive cleaning.
    #[command(visible_alias = "ui")]
    Tui,
}

/// Output format options
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Display as formatted table
    #[default]
    Table,
    /// Output as JSON
    Json,
    /// Simple list format
    List,
}

impl Cli {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
