//! Linux-specific cleanup rules

use super::{Category, CleanItem, CleanResult, CleanRule, RiskLevel};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Get all Linux-specific rules
pub fn get_linux_rules() -> Vec<Box<dyn CleanRule>> {
    vec![
        Box::new(AptCacheRule),
        Box::new(DnfCacheRule),
        Box::new(PacmanCacheRule),
        Box::new(SnapCacheRule),
        Box::new(FlatpakCacheRule),
        Box::new(JournalLogsRule),
        Box::new(UserCacheRule),
    ]
}

/// Calculate directory size recursively
fn dir_size(path: &std::path::Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Common function to clean items
fn clean_items(items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult> {
    let mut result = CleanResult::default();

    for item in items {
        let clean_result = if to_trash {
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
    }

    Ok(result)
}

/// APT cache rule (Debian/Ubuntu)
pub struct AptCacheRule;

impl CleanRule for AptCacheRule {
    fn name(&self) -> &str {
        "APT Package Cache"
    }

    fn category(&self) -> Category {
        Category::LinuxPackages
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Downloaded package files from APT (Debian/Ubuntu)"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        vec![PathBuf::from("/var/cache/apt/archives")]
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                // Only scan .deb files, not the lock file or partial directory
                if let Ok(entries) = std::fs::read_dir(&path) {
                    let mut total_size = 0u64;
                    let mut deb_count = 0;

                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_path = entry.path();
                        if entry_path.extension().map(|e| e == "deb").unwrap_or(false) {
                            if let Ok(metadata) = entry_path.metadata() {
                                total_size += metadata.len();
                                deb_count += 1;
                            }
                        }
                    }

                    if total_size > 0 {
                        items.push(CleanItem::new(
                            path,
                            total_size,
                            format!("APT cache ({} packages)", deb_count),
                            self.risk_level(),
                            self.category(),
                        ));
                    }
                }
            }
        }
        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], _to_trash: bool) -> anyhow::Result<CleanResult> {
        // For APT cache, we should use apt-get clean instead
        let mut result = CleanResult::default();

        for item in items {
            // Clean only .deb files
            if let Ok(entries) = std::fs::read_dir(&item.path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_path = entry.path();
                    if entry_path.extension().map(|e| e == "deb").unwrap_or(false) {
                        match std::fs::remove_file(&entry_path) {
                            Ok(_) => {
                                if let Ok(m) = entry_path.metadata() {
                                    result.bytes_freed += m.len();
                                }
                                result.cleaned_count += 1;
                            }
                            Err(e) => {
                                result.failed.push((entry_path, e.to_string()));
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

/// DNF/YUM cache rule (Fedora/RHEL)
pub struct DnfCacheRule;

impl CleanRule for DnfCacheRule {
    fn name(&self) -> &str {
        "DNF/YUM Package Cache"
    }

    fn category(&self) -> Category {
        Category::LinuxPackages
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Downloaded package files from DNF/YUM (Fedora/RHEL)"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        vec![
            PathBuf::from("/var/cache/dnf"),
            PathBuf::from("/var/cache/yum"),
        ]
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 0 {
                    items.push(CleanItem::new(
                        path,
                        size,
                        "DNF/YUM package cache",
                        self.risk_level(),
                        self.category(),
                    ));
                }
            }
        }
        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult> {
        clean_items(items, to_trash)
    }
}

/// Pacman cache rule (Arch Linux)
pub struct PacmanCacheRule;

impl CleanRule for PacmanCacheRule {
    fn name(&self) -> &str {
        "Pacman Package Cache"
    }

    fn category(&self) -> Category {
        Category::LinuxPackages
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Downloaded package files from Pacman (Arch Linux)"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        vec![PathBuf::from("/var/cache/pacman/pkg")]
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 0 {
                    items.push(CleanItem::new(
                        path,
                        size,
                        "Pacman package cache",
                        self.risk_level(),
                        self.category(),
                    ));
                }
            }
        }
        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult> {
        clean_items(items, to_trash)
    }
}

/// Snap cache rule
pub struct SnapCacheRule;

impl CleanRule for SnapCacheRule {
    fn name(&self) -> &str {
        "Snap Cache"
    }

    fn category(&self) -> Category {
        Category::LinuxPackages
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Cache files for Snap applications"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("snap"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for base_path in self.scan_paths() {
            if base_path.exists() {
                // Look for .cache directories in snap apps
                if let Ok(apps) = std::fs::read_dir(&base_path) {
                    for app_entry in apps.filter_map(|e| e.ok()) {
                        let app_path = app_entry.path();
                        if app_path.is_dir() {
                            // Check common/.cache
                            let cache_path = app_path.join("common/.cache");
                            if cache_path.exists() {
                                let size = dir_size(&cache_path);
                                if size > 1024 * 1024 {
                                    let app_name = app_path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    items.push(CleanItem::new(
                                        cache_path,
                                        size,
                                        format!("Snap cache: {}", app_name),
                                        self.risk_level(),
                                        self.category(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult> {
        clean_items(items, to_trash)
    }
}

/// Flatpak cache rule
pub struct FlatpakCacheRule;

impl CleanRule for FlatpakCacheRule {
    fn name(&self) -> &str {
        "Flatpak Cache"
    }

    fn category(&self) -> Category {
        Category::LinuxPackages
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Cache files for Flatpak applications"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".var/app"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for base_path in self.scan_paths() {
            if base_path.exists() {
                if let Ok(apps) = std::fs::read_dir(&base_path) {
                    for app_entry in apps.filter_map(|e| e.ok()) {
                        let app_path = app_entry.path();
                        if app_path.is_dir() {
                            let cache_path = app_path.join("cache");
                            if cache_path.exists() {
                                let size = dir_size(&cache_path);
                                if size > 1024 * 1024 {
                                    let app_name = app_path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    items.push(CleanItem::new(
                                        cache_path,
                                        size,
                                        format!("Flatpak cache: {}", app_name),
                                        self.risk_level(),
                                        self.category(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult> {
        clean_items(items, to_trash)
    }
}

/// Systemd journal logs rule
pub struct JournalLogsRule;

impl CleanRule for JournalLogsRule {
    fn name(&self) -> &str {
        "Systemd Journal Logs"
    }

    fn category(&self) -> Category {
        Category::System
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Systemd journal log files"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        vec![PathBuf::from("/var/log/journal")]
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 100 * 1024 * 1024 {
                    // Only show if > 100MB
                    items.push(CleanItem::new(
                        path,
                        size,
                        "Systemd journal logs (consider using journalctl --vacuum-size)",
                        self.risk_level(),
                        self.category(),
                    ));
                }
            }
        }
        Ok(items)
    }

    fn clean(&self, _items: &[CleanItem], _to_trash: bool) -> anyhow::Result<CleanResult> {
        // For journal logs, recommend using journalctl --vacuum-size instead
        Ok(CleanResult {
            cleaned_count: 0,
            bytes_freed: 0,
            failed: vec![],
            cancelled: false,
        })
    }
}

/// User cache rule (~/.cache)
pub struct UserCacheRule;

impl CleanRule for UserCacheRule {
    fn name(&self) -> &str {
        "User Cache Directory"
    }

    fn category(&self) -> Category {
        Category::System
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "User cache directory (~/.cache)"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(cache_dir) = dirs::cache_dir() {
            paths.push(cache_dir);
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();

        // Skip caches that are handled by other rules
        let skip_patterns = ["pip", "npm", "yarn", "cargo", "go"];

        for path in self.scan_paths() {
            if path.exists() {
                if let Ok(entries) = std::fs::read_dir(&path) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_path = entry.path();
                        let name = entry_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();

                        if skip_patterns.iter().any(|p| name.contains(p)) {
                            continue;
                        }

                        if entry_path.is_dir() {
                            let size = dir_size(&entry_path);
                            if size > 10 * 1024 * 1024 {
                                // > 10MB
                                items.push(CleanItem::new(
                                    entry_path,
                                    size,
                                    format!("Cache: {}", name),
                                    self.risk_level(),
                                    self.category(),
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult> {
        clean_items(items, to_trash)
    }
}
