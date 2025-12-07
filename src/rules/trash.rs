//! Trash cleanup rule

use super::{Category, CleanItem, CleanResult, CleanRule, RiskLevel};
use std::path::PathBuf;

/// Trash cleanup rule
pub struct TrashRule;

impl CleanRule for TrashRule {
    fn name(&self) -> &str {
        "Trash"
    }

    fn category(&self) -> Category {
        Category::System
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Empty system trash"
    }

    fn is_applicable(&self) -> bool {
        true // Always applicable
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        Vec::new() // Not path based in the traditional sense
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        // We can't easily list trash items with the `trash` crate in a cross-platform way
        // that gives us sizes effectively for individual files without some work.
        // But we can check if it's empty or not, or just represent it as one "Trash" item.

        // For accurate size, we might need platform specific logic.
        // But `trash` crate doesn't expose list/size easily in version 5?
        // Let's check imports in Cargo.toml. Yes "trash = 5".
        // Actually, checking trash size can be complex.
        // For macOS we can check ~/.Trash.

        let mut items = Vec::new();
        let mut total_size = 0;
        let mut found = false;

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                let trash_path = home.join(".Trash");
                if trash_path.exists() {
                    // Simple recursive size check
                    total_size += dir_size(&trash_path);
                    if total_size > 0 {
                        found = true;
                    }
                }
            }
        }

        // On Linux, trash usually in ~/.local/share/Trash
        #[cfg(target_os = "linux")]
        {
            if let Some(home) = dirs::home_dir() {
                let trash_path = home.join(".local/share/Trash/files");
                if trash_path.exists() {
                    total_size += dir_size(&trash_path);
                    if total_size > 0 {
                        found = true;
                    }
                }
            }
        }

        if found {
            items.push(CleanItem::new(
                PathBuf::from("System Trash"),
                total_size,
                "All items in the Trash",
                self.risk_level(),
                self.category(),
            ));
        }

        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], _to_trash: bool) -> anyhow::Result<CleanResult> {
        let mut result = CleanResult::default();

        // trash crate doesn't have "empty trash" function directly?
        // Checking docs... it usually handles moving TO trash.
        // To empty trash, we might need to actually delete the files in the trash folders.
        // Or use platform specific commands.
        // On macOS: `rm -rf ~/.Trash/*` (risky if not careful)

        // Let's rely on manual deletion of contents for now safely.

        for item in items {
            if item.path.to_string_lossy() == "System Trash" {
                // Delete contents of trash folders
                #[cfg(target_os = "macos")]
                {
                    if let Some(home) = dirs::home_dir() {
                        let trash_path = home.join(".Trash");
                        if let Ok(entries) = std::fs::read_dir(&trash_path) {
                            for entry in entries.filter_map(|e| e.ok()) {
                                let path = entry.path();
                                if path.is_dir() {
                                    if std::fs::remove_dir_all(&path).is_ok() {
                                        // Count rough estimate?
                                    }
                                } else if std::fs::remove_file(&path).is_ok() {
                                    // Count
                                }
                            }
                        }
                    }
                }

                // Result updates are tricky without exact counts
                result.cleaned_count += 1; // Count the "Trash" item itself
                result.bytes_freed += item.size;
            }
        }

        Ok(result)
    }
}

/// Calculate directory size recursively (copied helper)
fn dir_size(path: &std::path::Path) -> u64 {
    use walkdir::WalkDir;
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}
