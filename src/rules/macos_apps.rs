//! macOS application-specific cleanup rules

use super::{Category, CleanItem, CleanResult, CleanRule, RiskLevel};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Get all macOS application-specific rules
pub fn get_macos_app_rules() -> Vec<Box<dyn CleanRule>> {
    vec![
        Box::new(AppCacheRule),
        Box::new(AppLogsRule),
        Box::new(AppSupportCacheRule),
        Box::new(ContainerCacheRule),
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
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
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

/// Application cache rule (~/Library/Caches/<BundleID>)
pub struct AppCacheRule;

impl CleanRule for AppCacheRule {
    fn name(&self) -> &str {
        "Application Caches"
    }

    fn category(&self) -> Category {
        Category::MacApps
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Cache files for installed macOS applications"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Caches"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        
        // Skip system and already-handled caches
        let skip_patterns = [
            "com.apple.",
            "Homebrew",
            "CocoaPods",
            "CloudKit",
            "FamilyCircle",
            "Google",  // Often needed for Chrome etc
        ];

        for path in self.scan_paths() {
            if path.exists() {
                if let Ok(entries) = std::fs::read_dir(&path) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_path = entry.path();
                        let name = entry_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();

                        // Skip already handled or system caches
                        if skip_patterns.iter().any(|p| name.contains(p)) {
                            continue;
                        }

                        if entry_path.is_dir() {
                            let size = dir_size(&entry_path);
                            // Only show caches > 10MB
                            if size > 10 * 1024 * 1024 {
                                items.push(CleanItem::new(
                                    entry_path,
                                    size,
                                    format!("App cache: {}", name),
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

/// Application logs rule (~/Library/Logs/<AppName>)
pub struct AppLogsRule;

impl CleanRule for AppLogsRule {
    fn name(&self) -> &str {
        "Application Logs"
    }

    fn category(&self) -> Category {
        Category::MacApps
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Log files for installed macOS applications"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Logs"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        
        // Skip certain system logs
        let skip_patterns = ["DiagnosticReports", "CrashReporter"];

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
                            if size > 1024 * 1024 {
                                // > 1MB
                                items.push(CleanItem::new(
                                    entry_path,
                                    size,
                                    format!("App logs: {}", name),
                                    self.risk_level(),
                                    self.category(),
                                ));
                            }
                        } else if entry_path.is_file() {
                            // Individual log files
                            if let Ok(metadata) = entry_path.metadata() {
                                let size = metadata.len();
                                if size > 1024 * 1024 {
                                    items.push(CleanItem::new(
                                        entry_path,
                                        size,
                                        format!("Log file: {}", name),
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

/// Application Support cache rule (~/Library/Application Support/<App>/Cache)
pub struct AppSupportCacheRule;

impl CleanRule for AppSupportCacheRule {
    fn name(&self) -> &str {
        "Application Support Caches"
    }

    fn category(&self) -> Category {
        Category::MacApps
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Cache directories within Application Support folders"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Application Support"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        
        let cache_names = ["Cache", "Caches", "cache", "CachedData", "GPUCache", "ShaderCache"];

        for base_path in self.scan_paths() {
            if base_path.exists() {
                if let Ok(app_dirs) = std::fs::read_dir(&base_path) {
                    for app_entry in app_dirs.filter_map(|e| e.ok()) {
                        let app_path = app_entry.path();
                        if app_path.is_dir() {
                            // Look for cache directories inside each app folder
                            for cache_name in &cache_names {
                                let cache_path = app_path.join(cache_name);
                                if cache_path.exists() && cache_path.is_dir() {
                                    let size = dir_size(&cache_path);
                                    if size > 10 * 1024 * 1024 {
                                        // > 10MB
                                        let app_name = app_path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_default();
                                        items.push(CleanItem::new(
                                            cache_path,
                                            size,
                                            format!("{} cache", app_name),
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
        }
        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult> {
        clean_items(items, to_trash)
    }
}

/// Container cache rule (~/Library/Containers/<BundleID>/Data/Library/Caches)
pub struct ContainerCacheRule;

impl CleanRule for ContainerCacheRule {
    fn name(&self) -> &str {
        "Sandboxed App Caches"
    }

    fn category(&self) -> Category {
        Category::MacApps
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Cache files for sandboxed macOS applications"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Containers"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();

        for base_path in self.scan_paths() {
            if base_path.exists() {
                if let Ok(containers) = std::fs::read_dir(&base_path) {
                    for container_entry in containers.filter_map(|e| e.ok()) {
                        let container_path = container_entry.path();
                        if container_path.is_dir() {
                            let cache_path = container_path.join("Data/Library/Caches");
                            if cache_path.exists() && cache_path.is_dir() {
                                let size = dir_size(&cache_path);
                                if size > 5 * 1024 * 1024 {
                                    // > 5MB
                                    let container_name = container_path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    items.push(CleanItem::new(
                                        cache_path,
                                        size,
                                        format!("Container cache: {}", container_name),
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
