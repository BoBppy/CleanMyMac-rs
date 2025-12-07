//! Miscellaneous cleanup rules

use super::{Category, CleanItem, CleanResult, CleanRule, RiskLevel};
use std::path::PathBuf;
use walkdir::WalkDir;

/// .DS_Store cleanup rule
pub struct DsStoreRule;

impl CleanRule for DsStoreRule {
    fn name(&self) -> &str {
        ".DS_Store Files"
    }

    fn category(&self) -> Category {
        Category::System
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "macOS directory metadata files"
    }

    fn is_applicable(&self) -> bool {
        // Mostly relevant on macOS or if accessing folders touched by macOS
        true
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home);
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();

        if let Some(home) = dirs::home_dir() {
            // We restrict scan to specific areas to avoid scanning the entire disk deeply which is slow
            // Let's check Desktop, Documents, Downloads.
            let target_dirs = vec![
                home.join("Desktop"),
                home.join("Documents"),
                home.join("Downloads"),
                home.join("Public"),
                home.join("Pictures"),
                home.join("Music"),
                home.join("Movies"),
            ];

            for dir in target_dirs {
                if !dir.exists() {
                    continue;
                }

                for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() && entry.file_name() == ".DS_Store" {
                        if let Ok(metadata) = entry.metadata() {
                            items.push(CleanItem::new(
                                entry.path().to_path_buf(),
                                metadata.len(),
                                "Folder view settings",
                                self.risk_level(),
                                self.category(),
                            ));
                        }
                    }
                }
            }
        }

        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult> {
        let mut result = CleanResult::default();

        for item in items {
            let res = if to_trash {
                trash::delete(&item.path).map_err(|e| std::io::Error::other(e.to_string()))
            } else {
                std::fs::remove_file(&item.path)
            };

            match res {
                Ok(_) => {
                    result.cleaned_count += 1;
                    result.bytes_freed += item.size;
                }
                Err(e) => {
                    result.failed.push((item.path.clone(), e.to_string()));
                }
            }
        }

        Ok(result)
    }
}
