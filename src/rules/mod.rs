//! Rules module for defining cleanup rules
//!
//! This module contains the core trait for cleanup rules and implementations
//! for various platforms and development tools.

mod devtools;
mod docker;
mod heuristic;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod macos_apps;
mod misc;
mod trash;

pub use devtools::*;
pub use docker::*;
pub use heuristic::*;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(target_os = "macos")]
pub use macos_apps::*;
pub use misc::*;
pub use trash::*;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Risk level for cleanup operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk: cache files that can be safely deleted
    Low,
    /// Medium risk: may affect application performance temporarily
    Medium,
    /// High risk: requires explicit user confirmation
    High,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
        }
    }
}

/// Category of cleanup rules
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    /// System caches and logs
    System,
    /// Homebrew
    Brew,
    /// Xcode
    Xcode,
    /// Node.js (npm, yarn, pnpm)
    NodeJs,
    /// Python (pip, conda, uv)
    Python,
    /// Rust (cargo)
    Rust,
    /// Go
    Go,
    /// Java (gradle, maven)
    Java,
    /// Docker
    Docker,
    /// Android
    Android,
    /// Heuristically detected
    Heuristic,
    /// macOS Applications
    MacApps,
    /// Linux package managers
    LinuxPackages,
    /// Other
    Other(String),
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::System => write!(f, "System"),
            Category::Brew => write!(f, "Homebrew"),
            Category::Xcode => write!(f, "Xcode"),
            Category::NodeJs => write!(f, "Node.js"),
            Category::Python => write!(f, "Python"),
            Category::Rust => write!(f, "Rust"),
            Category::Go => write!(f, "Go"),
            Category::Java => write!(f, "Java"),
            Category::Docker => write!(f, "Docker"),
            Category::Android => write!(f, "Android"),
            Category::Heuristic => write!(f, "Heuristic"),
            Category::MacApps => write!(f, "macOS Apps"),
            Category::LinuxPackages => write!(f, "Linux Packages"),
            Category::Other(name) => write!(f, "{}", name),
        }
    }
}

/// A single item that can be cleaned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanItem {
    /// Path to the item
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// Description of what this item is
    pub description: String,
    /// Risk level of cleaning this item
    pub risk_level: RiskLevel,
    /// Category this item belongs to
    pub category: Category,
    /// Last modified time (Unix timestamp)
    pub last_modified: Option<i64>,
}

impl CleanItem {
    /// Create a new CleanItem
    pub fn new(
        path: PathBuf,
        size: u64,
        description: impl Into<String>,
        risk_level: RiskLevel,
        category: Category,
    ) -> Self {
        Self {
            path,
            size,
            description: description.into(),
            risk_level,
            category,
            last_modified: None,
        }
    }

    /// Set the last modified time
    pub fn with_last_modified(mut self, timestamp: i64) -> Self {
        self.last_modified = Some(timestamp);
        self
    }
}

/// Result of a cleanup operation
#[derive(Debug, Clone, Default)]
pub struct CleanResult {
    /// Number of items successfully cleaned
    pub cleaned_count: usize,
    /// Total bytes freed
    pub bytes_freed: u64,
    /// Items that failed to clean
    pub failed: Vec<(PathBuf, String)>,
    /// Whether the operation was cancelled
    pub cancelled: bool,
}

impl CleanResult {
    /// Create a cancelled result
    pub fn cancelled() -> Self {
        Self {
            cancelled: true,
            ..Default::default()
        }
    }

    /// Merge another result into this one
    pub fn merge(&mut self, other: CleanResult) {
        self.cleaned_count += other.cleaned_count;
        self.bytes_freed += other.bytes_freed;
        self.failed.extend(other.failed);
        self.cancelled = self.cancelled || other.cancelled;
    }
}

/// Trait for cleanup rules
pub trait CleanRule: Send + Sync {
    /// Name of the rule
    fn name(&self) -> &str;

    /// Category of the rule
    fn category(&self) -> Category;

    /// Risk level of the rule
    fn risk_level(&self) -> RiskLevel;

    /// Description of what this rule cleans
    fn description(&self) -> &str;

    /// Check if this rule is applicable to the current system
    fn is_applicable(&self) -> bool;

    /// Get paths that should be scanned
    fn scan_paths(&self) -> Vec<PathBuf>;

    /// Scan for cleanable items
    fn scan(&self) -> anyhow::Result<Vec<CleanItem>>;

    /// Clean the specified items
    fn clean(&self, items: &[CleanItem], to_trash: bool) -> anyhow::Result<CleanResult>;
}

/// Get all available rules for the current platform
pub fn get_all_rules() -> Vec<Box<dyn CleanRule>> {
    let mut rules: Vec<Box<dyn CleanRule>> = Vec::new();

    // Add macOS-specific rules
    #[cfg(target_os = "macos")]
    {
        rules.extend(macos::get_macos_rules());
        rules.extend(macos_apps::get_macos_app_rules());
    }

    // Add Linux-specific rules
    #[cfg(target_os = "linux")]
    {
        rules.extend(linux::get_linux_rules());
    }

    // Add cross-platform dev tools rules
    rules.extend(devtools::get_devtools_rules());

    // Add Docker rule
    rules.push(Box::new(docker::DockerRule));

    // Add Trash rule
    rules.push(Box::new(trash::TrashRule));

    // Add Misc rules
    rules.push(Box::new(misc::DsStoreRule));

    // Add heuristic detector
    rules.push(Box::new(heuristic::HeuristicRule::default()));

    rules
}

/// Get rules filtered by category
pub fn get_rules_by_category(categories: &[String]) -> Vec<Box<dyn CleanRule>> {
    get_all_rules()
        .into_iter()
        .filter(|rule| {
            let cat_str = rule.category().to_string().to_lowercase();
            categories.iter().any(|c| c.to_lowercase() == cat_str)
        })
        .collect()
}
