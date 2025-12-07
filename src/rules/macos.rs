//! macOS-specific cleanup rules

use super::{Category, CleanItem, CleanResult, CleanRule, RiskLevel};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Get all macOS-specific rules
pub fn get_macos_rules() -> Vec<Box<dyn CleanRule>> {
    vec![
        Box::new(HomebrewRule),
        Box::new(XcodeDerivedDataRule),
        Box::new(XcodeArchivesRule),
        Box::new(XcodeDeviceSupportRule),
        Box::new(CocoaPodsRule),
        Box::new(SimulatorRule),
        Box::new(MacOSCacheRule),
        Box::new(MacOSLogsRule),
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

/// Homebrew cache rule
pub struct HomebrewRule;

impl CleanRule for HomebrewRule {
    fn name(&self) -> &str {
        "Homebrew Cache"
    }

    fn category(&self) -> Category {
        Category::Brew
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Homebrew downloaded packages and caches"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Caches/Homebrew"));
        }
        paths
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
                        "Homebrew download cache",
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

/// Xcode DerivedData rule
pub struct XcodeDerivedDataRule;

impl CleanRule for XcodeDerivedDataRule {
    fn name(&self) -> &str {
        "Xcode DerivedData"
    }

    fn category(&self) -> Category {
        Category::Xcode
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Xcode build artifacts and intermediate files"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Developer/Xcode/DerivedData"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                // Scan individual project folders
                if let Ok(entries) = std::fs::read_dir(&path) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            let size = dir_size(&entry_path);
                            if size > 0 {
                                let name = entry_path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                items.push(CleanItem::new(
                                    entry_path,
                                    size,
                                    format!("Xcode build data for {}", name),
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

/// Xcode Archives rule
pub struct XcodeArchivesRule;

impl CleanRule for XcodeArchivesRule {
    fn name(&self) -> &str {
        "Xcode Archives"
    }

    fn category(&self) -> Category {
        Category::Xcode
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Old Xcode archive files"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Developer/Xcode/Archives"));
        }
        paths
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
                        "Xcode archive files",
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

/// Xcode Device Support rule
pub struct XcodeDeviceSupportRule;

impl CleanRule for XcodeDeviceSupportRule {
    fn name(&self) -> &str {
        "Xcode Device Support"
    }

    fn category(&self) -> Category {
        Category::Xcode
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "iOS/watchOS device support files for debugging"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Developer/Xcode/iOS DeviceSupport"));
            paths.push(home.join("Library/Developer/Xcode/watchOS DeviceSupport"));
        }
        paths
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
                        "Device support symbols",
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

/// CocoaPods cache rule
pub struct CocoaPodsRule;

impl CleanRule for CocoaPodsRule {
    fn name(&self) -> &str {
        "CocoaPods Cache"
    }

    fn category(&self) -> Category {
        Category::Xcode
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "CocoaPods spec and download cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Caches/CocoaPods"));
        }
        paths
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
                        "CocoaPods cache",
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

/// iOS Simulator rule
pub struct SimulatorRule;

impl CleanRule for SimulatorRule {
    fn name(&self) -> &str {
        "iOS Simulators"
    }

    fn category(&self) -> Category {
        Category::Xcode
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::High
    }

    fn description(&self) -> &str {
        "iOS/watchOS/tvOS simulator data"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Developer/CoreSimulator/Devices"));
        }
        paths
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
                        "iOS Simulator data (will reset all simulators)",
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

/// macOS Cache rule
pub struct MacOSCacheRule;

impl CleanRule for MacOSCacheRule {
    fn name(&self) -> &str {
        "macOS User Caches"
    }

    fn category(&self) -> Category {
        Category::System
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "User application caches in ~/Library/Caches"
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
        for path in self.scan_paths() {
            if path.exists() {
                // Scan individual app caches, skip certain system caches
                let skip_patterns = ["com.apple.", "CloudKit", "FamilyCircle"];
                
                if let Ok(entries) = std::fs::read_dir(&path) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let entry_path = entry.path();
                        let name = entry_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        
                        // Skip system caches
                        if skip_patterns.iter().any(|p| name.starts_with(p)) {
                            continue;
                        }
                        
                        if entry_path.is_dir() {
                            let size = dir_size(&entry_path);
                            if size > 1024 * 1024 {
                                // Only show caches > 1MB
                                items.push(CleanItem::new(
                                    entry_path,
                                    size,
                                    format!("Cache for {}", name),
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

/// macOS Logs rule
pub struct MacOSLogsRule;

impl CleanRule for MacOSLogsRule {
    fn name(&self) -> &str {
        "macOS User Logs"
    }

    fn category(&self) -> Category {
        Category::System
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "User application logs in ~/Library/Logs"
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
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 0 {
                    items.push(CleanItem::new(
                        path,
                        size,
                        "User application logs",
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
