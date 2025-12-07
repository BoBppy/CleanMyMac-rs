//! CleanMyMac-rs - A cross-platform system cleaner
//!
//! A powerful tool for cleaning caches, temporary files, and development artifacts
//! on macOS and Linux systems. Built with Rust for performance and safety.

use cleanmymac_rs::{
    cleaner::Cleaner,
    config::Config,
    rules::{get_all_rules, get_rules_by_category},
    scanner::{FileScanner, ScanSummary, StorageAnalyzer},
    ui::{Cli, Commands, OutputFormat, tui::App},
};
use colored::*;
use dialoguer::Confirm;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let cli = Cli::parse_args();

    // Setup logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    // Disable colors if requested
    if cli.no_color {
        colored::control::set_override(false);
    }

    // Load configuration
    let config = if let Some(config_path) = &cli.config {
        Config::load_from(config_path)?
    } else {
        Config::load_or_default()
    };

    // Handle commands
    match cli.command {
        Commands::Scan {
            categories,
            format,
            min_size,
        } => {
            run_scan(categories, format, min_size)?;
        }
        Commands::Clean {
            categories,
            dry_run,
            yes,
            permanent,
            quiet,
        } => {
            run_clean(categories, dry_run, yes, permanent, quiet, &config)?;
        }
        Commands::Analyze { path, depth, top } => {
            run_analyze(path, depth, top)?;
        }
        Commands::List { category, detailed } => {
            run_list(category, detailed)?;
        }
        Commands::Config { init, show, path } => {
            run_config(init, show, path)?;
        }
        Commands::Tui => {
            run_tui()?;
        }
    }

    Ok(())
}

/// Run the scan command
fn run_scan(
    categories: Option<Vec<String>>,
    format: OutputFormat,
    _min_size: Option<String>,
) -> anyhow::Result<()> {
    println!("{}", "\n🔍 Scanning for cleanable files...\n".cyan().bold());

    let rules = if let Some(cats) = categories {
        get_rules_by_category(&cats)
    } else {
        get_all_rules()
    };

    let scanner = FileScanner::new(rules);
    let items = scanner.scan()?;

    if items.is_empty() {
        println!("\n{}", "✨ No cleanable files found!".green());
        return Ok(());
    }

    let summary = ScanSummary::from_items(items);

    match format {
        OutputFormat::Table => {
            print_summary_table(&summary);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&summary.by_category)?;
            println!("{}", json);
        }
        OutputFormat::List => {
            for (category, items) in &summary.by_category {
                println!("\n{}:", category.bold());
                for item in items {
                    println!(
                        "  {} ({})",
                        item.path.display(),
                        bytesize::ByteSize::b(item.size)
                    );
                }
            }
        }
    }

    Ok(())
}

/// Print summary as a table
fn print_summary_table(summary: &ScanSummary) {
    println!("\n{}", "📊 Scan Results".bold());
    println!("{}", "═".repeat(60));

    for (category, items) in &summary.by_category {
        let cat_size: u64 = items.iter().map(|i| i.size).sum();
        println!(
            "\n{} {} ({} items, {})",
            "▸".cyan(),
            category.bold(),
            items.len(),
            bytesize::ByteSize::b(cat_size).to_string().green()
        );

        for item in items.iter().take(5) {
            let risk_indicator = match item.risk_level {
                cleanmymac_rs::rules::RiskLevel::Low => "●".green(),
                cleanmymac_rs::rules::RiskLevel::Medium => "●".yellow(),
                cleanmymac_rs::rules::RiskLevel::High => "●".red(),
            };
            println!(
                "    {} {} ({})",
                risk_indicator,
                item.path.display(),
                bytesize::ByteSize::b(item.size)
            );
        }

        if items.len() > 5 {
            println!("    {} ...and {} more", "".dimmed(), items.len() - 5);
        }
    }

    println!("\n{}", "═".repeat(60));
    println!(
        "{} {} items, {}",
        "Total:".bold(),
        summary.total_items,
        bytesize::ByteSize::b(summary.total_size)
            .to_string()
            .green()
            .bold()
    );
}

/// Run the clean command
fn run_clean(
    categories: Option<Vec<String>>,
    dry_run: bool,
    yes: bool,
    permanent: bool,
    _quiet: bool,
    config: &Config,
) -> anyhow::Result<()> {
    println!("{}", "\n🧹 Preparing to clean...\n".cyan().bold());

    let rules = if let Some(cats) = categories {
        get_rules_by_category(&cats)
    } else {
        get_all_rules()
    };

    let scanner = FileScanner::new(rules);
    let items = scanner.scan()?;

    if items.is_empty() {
        println!("\n{}", "✨ Nothing to clean!".green());
        return Ok(());
    }

    // Show preview
    let cleaner = Cleaner::new()
        .use_trash(!permanent && config.general.use_trash)
        .confirm_high_risk(config.general.confirm_high_risk)
        .dry_run(dry_run);

    cleaner.preview(&items);

    // Confirm unless --yes was passed or dry run
    if !yes && !dry_run {
        let total_size = bytesize::ByteSize::b(items.iter().map(|i| i.size).sum());
        let confirm = Confirm::new()
            .with_prompt(format!(
                "\nDo you want to clean {} items ({})? {}",
                items.len(),
                total_size,
                if permanent {
                    "(PERMANENT)"
                } else {
                    "(to trash)"
                }
            ))
            .default(false)
            .interact()
            .unwrap_or(false);

        if !confirm {
            println!("{}", "\n❌ Cancelled.".yellow());
            return Ok(());
        }
    }

    // Execute cleaning
    let result = cleaner.clean(&items)?;

    // Show results
    if result.cancelled {
        println!("{}", "\n❌ Cleaning cancelled.".yellow());
    } else {
        println!(
            "\n{} Cleaned {} items, freed {}",
            "✅".green(),
            result.cleaned_count,
            bytesize::ByteSize::b(result.bytes_freed)
                .to_string()
                .green()
                .bold()
        );

        if !result.failed.is_empty() {
            println!("\n{}", "⚠️  Some items failed to clean:".yellow());
            for (path, error) in &result.failed {
                println!("    {} {}: {}", "✗".red(), path.display(), error);
            }
        }
    }

    Ok(())
}

/// Run the analyze command
fn run_analyze(path: Option<String>, depth: usize, top: usize) -> anyhow::Result<()> {
    let target_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
    };

    println!(
        "{} {}\n",
        "📊 Analyzing:".cyan().bold(),
        target_path.display()
    );

    let analyzer = StorageAnalyzer::new().with_max_depth(depth).with_top_n(top);

    let info = analyzer.analyze(&target_path)?;

    println!("{}", "Storage Analysis".bold());
    println!("{}", "═".repeat(60));
    println!(
        "Total size: {}",
        bytesize::ByteSize::b(info.total_size).to_string().green()
    );
    println!("Files: {}", info.file_count);
    println!("Directories: {}", info.dir_count);

    if !info.largest_files.is_empty() {
        println!("\n{}", "Largest Files:".bold());
        for (path, size) in &info.largest_files {
            println!(
                "  {} {} ({})",
                "•".cyan(),
                path.display(),
                bytesize::ByteSize::b(*size).to_string().yellow()
            );
        }
    }

    if !info.by_extension.is_empty() {
        println!("\n{}", "Size by Extension (top 10):".bold());
        let mut extensions: Vec<_> = info.by_extension.iter().collect();
        extensions.sort_by(|a, b| b.1.cmp(a.1));
        for (ext, size) in extensions.iter().take(10) {
            println!(
                "  .{}: {}",
                ext,
                bytesize::ByteSize::b(**size).to_string().green()
            );
        }
    }

    Ok(())
}

/// Run the list command
fn run_list(category: Option<String>, detailed: bool) -> anyhow::Result<()> {
    println!("{}", "\n📋 Available Cleanup Rules\n".cyan().bold());

    let rules = if let Some(cat) = category {
        get_rules_by_category(&[cat])
    } else {
        get_all_rules()
    };

    if rules.is_empty() {
        println!("{}", "No rules found for the specified category.".yellow());
        return Ok(());
    }

    for rule in &rules {
        let risk_indicator = match rule.risk_level() {
            cleanmymac_rs::rules::RiskLevel::Low => "●".green(),
            cleanmymac_rs::rules::RiskLevel::Medium => "●".yellow(),
            cleanmymac_rs::rules::RiskLevel::High => "●".red(),
        };

        let applicable = if rule.is_applicable() {
            "✓".green()
        } else {
            "✗".dimmed()
        };

        println!(
            "{} {} {} [{}] ({})",
            applicable,
            risk_indicator,
            rule.name().bold(),
            rule.category(),
            rule.risk_level()
        );

        if detailed {
            println!("    {}", rule.description().dimmed());
            let paths: Vec<_> = rule
                .scan_paths()
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            if !paths.is_empty() {
                println!("    Paths: {}", paths.join(", ").dimmed());
            }
            println!();
        }
    }

    println!(
        "\n{} {} rules available ({} applicable)",
        "Total:".bold(),
        rules.len(),
        rules.iter().filter(|r| r.is_applicable()).count()
    );

    Ok(())
}

/// Run the config command
fn run_config(init: bool, show: bool, path: Option<String>) -> anyhow::Result<()> {
    if init {
        let config_path = if let Some(p) = path {
            std::path::PathBuf::from(p)
        } else {
            Config::default_path()?
        };

        let config = Config::default();
        config.save_to(&config_path)?;
        println!(
            "{} Configuration saved to: {}",
            "✅".green(),
            config_path.display()
        );
    } else if show {
        let config = if let Some(p) = path {
            Config::load_from(&p)?
        } else {
            Config::load_or_default()
        };

        let toml_str = toml::to_string_pretty(&config)?;
        println!("{}", "Current Configuration:".bold());
        println!("{}", "═".repeat(60));
        println!("{}", toml_str);
    } else {
        println!("{}", "Configuration Commands:".bold());
        println!(
            "  {} Initialize default configuration",
            "cleanmymac-rs config --init".cyan()
        );
        println!(
            "  {} Show current configuration",
            "cleanmymac-rs config --show".cyan()
        );
        println!(
            "  {} Initialize at custom path",
            "cleanmymac-rs config --init --path <PATH>".cyan()
        );
    }

    Ok(())
}

/// Run TUI mode
fn run_tui() -> anyhow::Result<()> {
    let mut app = App::new();
    app.run()?;
    Ok(())
}
