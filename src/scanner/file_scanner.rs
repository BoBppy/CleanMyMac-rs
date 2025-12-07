//! Parallel file scanner using rayon

use crate::rules::{CleanItem, CleanRule};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

/// File scanner for scanning cleanable items
pub struct FileScanner {
    /// Rules to use for scanning
    rules: Vec<Box<dyn CleanRule>>,
}

impl FileScanner {
    /// Create a new file scanner with the given rules
    pub fn new(rules: Vec<Box<dyn CleanRule>>) -> Self {
        Self { rules }
    }

    /// Scan all rules and return cleanable items
    pub fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let items: Arc<Mutex<Vec<CleanItem>>> = Arc::new(Mutex::new(Vec::new()));

        let pb = ProgressBar::new(self.rules.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}",
                )
                .unwrap_or_else(|_| ProgressStyle::default_bar()),
        );

        // Scan rules in parallel
        self.rules.par_iter().for_each(|rule| {
            if rule.is_applicable() {
                pb.set_message(format!("Scanning: {}", rule.name()));
                match rule.scan() {
                    Ok(found_items) => {
                        let mut items_guard = items.lock().unwrap();
                        items_guard.extend(found_items);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to scan {}: {}", rule.name(), e);
                    }
                }
            }
            pb.inc(1);
        });

        pb.finish_with_message("Scan complete");

        let result = Arc::try_unwrap(items)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?
            .into_inner()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?;

        Ok(result)
    }

    /// Scan rules without progress bar (for non-interactive use)
    pub fn scan_quiet(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut all_items = Vec::new();

        for rule in &self.rules {
            if rule.is_applicable() {
                match rule.scan() {
                    Ok(items) => all_items.extend(items),
                    Err(e) => {
                        tracing::warn!("Failed to scan {}: {}", rule.name(), e);
                    }
                }
            }
        }

        Ok(all_items)
    }

    /// Scan rules in parallel without progress bar
    pub fn scan_parallel_quiet(&self) -> anyhow::Result<Vec<CleanItem>> {
        let items: Arc<Mutex<Vec<CleanItem>>> = Arc::new(Mutex::new(Vec::new()));

        self.rules.par_iter().for_each(|rule| {
            if rule.is_applicable() {
                match rule.scan() {
                    Ok(found_items) => {
                        let mut items_guard = items.lock().unwrap();
                        items_guard.extend(found_items);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to scan {}: {}", rule.name(), e);
                    }
                }
            }
        });

        let result = Arc::try_unwrap(items)
            .map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?
            .into_inner()
            .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))?;

        Ok(result)
    }
}

/// Summary of scan results
#[derive(Debug, Clone, Default)]
pub struct ScanSummary {
    /// Total number of items found
    pub total_items: usize,
    /// Total size in bytes
    pub total_size: u64,
    /// Items grouped by category
    pub by_category: std::collections::HashMap<String, Vec<CleanItem>>,
}

impl ScanSummary {
    /// Create a summary from a list of items
    pub fn from_items(items: Vec<CleanItem>) -> Self {
        use std::collections::HashMap;

        let mut by_category: HashMap<String, Vec<CleanItem>> = HashMap::new();
        let mut total_size = 0u64;

        for item in items {
            total_size += item.size;
            let category_name = item.category.to_string();
            by_category.entry(category_name).or_default().push(item);
        }

        let total_items = by_category.values().map(|v| v.len()).sum();

        Self {
            total_items,
            total_size,
            by_category,
        }
    }
}
