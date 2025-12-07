//! Cleaner module for executing cleanup operations

use crate::rules::{CleanItem, CleanResult, RiskLevel};
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;

/// Cleaner for executing cleanup operations
pub struct Cleaner {
    /// Whether to use trash instead of permanent deletion
    use_trash: bool,
    /// Whether to confirm high-risk operations
    confirm_high_risk: bool,
    /// Dry run mode (no actual deletion)
    dry_run: bool,
}

impl Default for Cleaner {
    fn default() -> Self {
        Self {
            use_trash: true,
            confirm_high_risk: true,
            dry_run: false,
        }
    }
}

impl Cleaner {
    /// Create a new cleaner
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to use trash
    pub fn use_trash(mut self, value: bool) -> Self {
        self.use_trash = value;
        self
    }

    /// Set whether to confirm high-risk operations
    pub fn confirm_high_risk(mut self, value: bool) -> Self {
        self.confirm_high_risk = value;
        self
    }

    /// Set dry run mode
    pub fn dry_run(mut self, value: bool) -> Self {
        self.dry_run = value;
        self
    }

    /// Clean the specified items
    pub fn clean(&self, items: &[CleanItem]) -> anyhow::Result<CleanResult> {
        let mut result = CleanResult::default();

        // Filter out items that need confirmation
        let (high_risk, normal): (Vec<_>, Vec<_>) = items
            .iter()
            .partition(|item| item.risk_level == RiskLevel::High);

        // Handle high-risk items first
        if !high_risk.is_empty() && self.confirm_high_risk {
            println!("\n{}", "⚠️  High-risk items detected:".yellow().bold());
            for item in &high_risk {
                println!(
                    "  {} {} ({})",
                    "•".red(),
                    item.path.display(),
                    bytesize::ByteSize::b(item.size)
                );
            }

            let confirm = Confirm::new()
                .with_prompt("Do you want to clean these high-risk items?")
                .default(false)
                .interact()
                .unwrap_or(false);

            if confirm {
                let high_risk_result = self.clean_items(&high_risk)?;
                result.merge(high_risk_result);
            } else {
                println!("{}", "Skipping high-risk items.".yellow());
            }
        } else if !high_risk.is_empty() {
            let high_risk_result = self.clean_items(&high_risk)?;
            result.merge(high_risk_result);
        }

        // Clean normal items
        if !normal.is_empty() {
            let normal_result = self.clean_items(&normal)?;
            result.merge(normal_result);
        }

        Ok(result)
    }

    /// Clean a list of items with progress bar
    fn clean_items(&self, items: &[&CleanItem]) -> anyhow::Result<CleanResult> {
        let mut result = CleanResult::default();

        if self.dry_run {
            println!("\n{}", "Dry run mode - no files will be deleted:".cyan());
            for item in items {
                println!(
                    "  {} {} ({})",
                    "Would delete:".cyan(),
                    item.path.display(),
                    bytesize::ByteSize::b(item.size)
                );
                result.bytes_freed += item.size;
                result.cleaned_count += 1;
            }
            return Ok(result);
        }

        let pb = ProgressBar::new(items.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
        );

        for item in items {
            pb.set_message(format!("Cleaning: {}", item.path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()));

            let clean_result = if self.use_trash {
                trash::delete(&item.path)
                    .map_err(|e| std::io::Error::other(e.to_string()))
            } else if item.path.is_dir() {
                std::fs::remove_dir_all(&item.path)
            } else {
                std::fs::remove_file(&item.path)
            };

            match clean_result {
                Ok(_) => {
                    result.cleaned_count += 1;
                    result.bytes_freed += item.size;
                }
                Err(e) => {
                    result.failed.push((item.path.clone(), e.to_string()));
                }
            }

            pb.inc(1);
        }

        pb.finish_with_message("Clean complete");
        Ok(result)
    }

    /// Preview what would be cleaned
    pub fn preview(&self, items: &[CleanItem]) {
        use std::collections::HashMap;

        let mut by_category: HashMap<String, Vec<&CleanItem>> = HashMap::new();
        let mut total_size = 0u64;

        for item in items {
            total_size += item.size;
            let category_name = item.category.to_string();
            by_category.entry(category_name).or_default().push(item);
        }

        println!("\n{}", "📊 Scan Results:".bold());
        println!("{}", "═".repeat(60));

        for (category, cat_items) in &by_category {
            let cat_size: u64 = cat_items.iter().map(|i| i.size).sum();
            println!(
                "\n{} {} ({} items, {})",
                "▸".cyan(),
                category.bold(),
                cat_items.len(),
                bytesize::ByteSize::b(cat_size).to_string().green()
            );

            for item in cat_items.iter().take(5) {
                // let risk_color = match item.risk_level {
                //     RiskLevel::Low => "green",
                //     RiskLevel::Medium => "yellow",
                //     RiskLevel::High => "red",
                // };
                println!(
                    "    {} {} ({})",
                    match item.risk_level {
                        RiskLevel::Low => "●".green(),
                        RiskLevel::Medium => "●".yellow(),
                        RiskLevel::High => "●".red(),
                    },
                    item.path.display(),
                    bytesize::ByteSize::b(item.size)
                );
            }

            if cat_items.len() > 5 {
                println!("    {} ...and {} more", "".dimmed(), cat_items.len() - 5);
            }
        }

        println!("\n{}", "═".repeat(60));
        println!(
            "{} {} items, {}",
            "Total:".bold(),
            items.len(),
            bytesize::ByteSize::b(total_size).to_string().green().bold()
        );
    }
}
