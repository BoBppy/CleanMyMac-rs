//! Heuristic detection for automatically discovering cache directories

use super::{Category, CleanItem, CleanResult, CleanRule, RiskLevel};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

/// Default size threshold (100MB)
const DEFAULT_SIZE_THRESHOLD: u64 = 100 * 1024 * 1024;

/// Default stale days threshold
const DEFAULT_STALE_DAYS: u32 = 30;

/// Patterns that indicate a cache directory
const CACHE_PATTERNS: &[&str] = &[
    "cache",
    "Cache",
    ".cache",
    "caches",
    "Caches",
    "tmp",
    "temp",
    "Temp",
    "Temporary",
];

/// Patterns that indicate temporary files
const TEMP_EXTENSIONS: &[&str] = &[
    "tmp", "temp", "log", "bak", "old", "orig", "swp", "swo",
];

/// Heuristic detection rule
#[derive(Debug)]
pub struct HeuristicRule {
    /// Size threshold in bytes
    size_threshold: u64,
    /// Stale days threshold
    stale_days: u32,
}

impl Default for HeuristicRule {
    fn default() -> Self {
        Self {
            size_threshold: DEFAULT_SIZE_THRESHOLD,
            stale_days: DEFAULT_STALE_DAYS,
        }
    }
}

impl HeuristicRule {
    /// Create a new heuristic rule with custom thresholds
    pub fn new(size_threshold: u64, stale_days: u32) -> Self {
        Self {
            size_threshold,
            stale_days,
        }
    }

    /// Check if a directory name matches cache patterns
    fn is_cache_name(name: &str) -> bool {
        let lower = name.to_lowercase();
        CACHE_PATTERNS.iter().any(|p| lower.contains(&p.to_lowercase()))
    }

    /// Check if a file has a temporary extension
    fn is_temp_file(name: &str) -> bool {
        if let Some(ext) = name.rsplit('.').next() {
            TEMP_EXTENSIONS.contains(&ext.to_lowercase().as_str())
        } else {
            false
        }
    }

    /// Calculate directory size
    fn dir_size(path: &std::path::Path) -> u64 {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    }

    /// Check if a path was last modified before the stale threshold
    fn is_stale(&self, path: &std::path::Path) -> bool {
        if let Ok(metadata) = path.metadata() {
            if let Ok(modified) = metadata.modified() {
                let threshold = SystemTime::now() - Duration::from_secs(self.stale_days as u64 * 24 * 60 * 60);
                return modified < threshold;
            }
        }
        false
    }

    /// Scan a directory for heuristically detected caches
    fn scan_directory(&self, base_path: &std::path::Path) -> Vec<CleanItem> {
        let mut items = Vec::new();

        if !base_path.exists() || !base_path.is_dir() {
            return items;
        }

        // Look for cache directories
        for entry in WalkDir::new(base_path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            // Skip if we can't read
            if path.metadata().is_err() {
                continue;
            }

            // Check if this is a cache directory by name
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if Self::is_cache_name(name) {
                        let size = Self::dir_size(path);
                        if size >= self.size_threshold {
                            let is_stale = self.is_stale(path);
                            let risk = if is_stale {
                                RiskLevel::Low
                            } else {
                                RiskLevel::Medium
                            };
                            
                            items.push(CleanItem::new(
                                path.to_path_buf(),
                                size,
                                format!(
                                    "Heuristically detected cache{}",
                                    if is_stale { " (stale)" } else { "" }
                                ),
                                risk,
                                Category::Heuristic,
                            ));
                        }
                    }
                }
            }
        }

        items
    }
}

impl CleanRule for HeuristicRule {
    fn name(&self) -> &str {
        "Heuristic Detection"
    }

    fn category(&self) -> Category {
        Category::Heuristic
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Automatically detected cache and temporary directories"
    }

    fn is_applicable(&self) -> bool {
        true
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        
        // Common locations to scan for caches
        if let Some(home) = dirs::home_dir() {
            paths.push(home.clone());
            
            // Common project directories
            for dir in &["Projects", "projects", "Code", "code", "Development", "dev", "src"] {
                let p = home.join(dir);
                if p.exists() {
                    paths.push(p);
                }
            }
        }
        
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        
        // Scan home directory (with limited depth)
        if let Some(home) = dirs::home_dir() {
            // Scan direct children of home for cache directories
            if let Ok(entries) = std::fs::read_dir(&home) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            // Skip already-handled directories
                            if name.starts_with('.') && !Self::is_cache_name(name) {
                                continue;
                            }
                            
                            if Self::is_cache_name(name) {
                                let size = Self::dir_size(&path);
                                if size >= self.size_threshold {
                                    items.push(CleanItem::new(
                                        path,
                                        size,
                                        "Heuristically detected cache directory",
                                        RiskLevel::Medium,
                                        Category::Heuristic,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            
            // Scan project directories for large temp/cache directories
            for dir in &["Projects", "projects", "Code", "code", "Development", "dev"] {
                let project_dir = home.join(dir);
                if project_dir.exists() {
                    items.extend(self.scan_directory(&project_dir));
                }
            }
        }

        // Deduplicate by path
        items.sort_by(|a, b| a.path.cmp(&b.path));
        items.dedup_by(|a, b| a.path == b.path);

        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult> {
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
}

/// Classification of a detected cache
#[derive(Debug, Clone)]
pub struct CacheClassification {
    /// Path to the cache
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Why it was classified as cache
    pub reason: String,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
    /// Whether the cache appears stale
    pub is_stale: bool,
}

/// Heuristic detector for discovering unknown caches
pub struct HeuristicDetector {
    size_threshold: u64,
    stale_days: u32,
}

impl Default for HeuristicDetector {
    fn default() -> Self {
        Self {
            size_threshold: DEFAULT_SIZE_THRESHOLD,
            stale_days: DEFAULT_STALE_DAYS,
        }
    }
}

impl HeuristicDetector {
    /// Create a new detector with custom settings
    pub fn new(size_threshold_mb: u64, stale_days: u32) -> Self {
        Self {
            size_threshold: size_threshold_mb * 1024 * 1024,
            stale_days,
        }
    }

    /// Analyze a path and classify it
    pub fn analyze(&self, path: &std::path::Path) -> Option<CacheClassification> {
        if !path.exists() || !path.is_dir() {
            return None;
        }

        let name = path.file_name()?.to_str()?;
        
        // Check if name matches cache patterns
        if !HeuristicRule::is_cache_name(name) {
            return None;
        }

        let size = HeuristicRule::dir_size(path);
        if size < self.size_threshold {
            return None;
        }

        // Calculate confidence based on various factors
        let mut confidence: f64 = 0.5;
        
        // Exact match increases confidence
        if name.to_lowercase() == "cache" || name.to_lowercase() == "caches" {
            confidence += 0.3;
        }
        
        // Check staleness
        let is_stale = if let Ok(metadata) = path.metadata() {
            if let Ok(modified) = metadata.modified() {
                let threshold = SystemTime::now() - Duration::from_secs(self.stale_days as u64 * 24 * 60 * 60);
                if modified < threshold {
                    confidence += 0.1;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        Some(CacheClassification {
            path: path.to_path_buf(),
            size,
            reason: format!("Directory name '{}' matches cache pattern", name),
            confidence: confidence.min(1.0),
            is_stale,
        })
    }

    /// Discover potential caches in a directory
    pub fn discover(&self, root: &std::path::Path) -> Vec<CacheClassification> {
        let mut results = Vec::new();

        for entry in WalkDir::new(root)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Some(classification) = self.analyze(entry.path()) {
                results.push(classification);
            }
        }

        results
    }
}
