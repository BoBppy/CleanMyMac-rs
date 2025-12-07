//! Configuration management for CleanMyMac-rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    /// General settings
    pub general: GeneralConfig,
    /// Category settings
    pub categories: CategoryConfig,
    /// Heuristic detection settings
    pub heuristic: HeuristicConfig,
    /// Risk confirmation settings
    pub risk: RiskConfig,
    /// Ignore settings
    pub ignore: IgnoreConfig,
}


/// General configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Whether to use trash instead of permanent deletion
    pub use_trash: bool,
    /// Whether to scan hidden files
    pub scan_hidden: bool,
    /// Number of parallel threads (0 = auto)
    pub parallel_threads: usize,
    /// Whether to confirm high-risk operations
    pub confirm_high_risk: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            use_trash: true,
            scan_hidden: true,
            parallel_threads: 0,
            confirm_high_risk: true,
        }
    }
}

/// Category configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CategoryConfig {
    /// Enabled cleanup categories
    pub enabled: Vec<String>,
}

impl Default for CategoryConfig {
    fn default() -> Self {
        Self {
            enabled: vec![
                "system".to_string(),
                "brew".to_string(),
                "xcode".to_string(),
                "npm".to_string(),
                "pip".to_string(),
                "cargo".to_string(),
                "docker".to_string(),
            ],
        }
    }
}

/// Heuristic detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HeuristicConfig {
    /// Whether heuristic detection is enabled
    pub enabled: bool,
    /// Size threshold in MB for detecting large cache directories
    pub size_threshold_mb: u64,
    /// Number of days after which a file is considered stale
    pub stale_days: u32,
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            size_threshold_mb: 100,
            stale_days: 30,
        }
    }
}

/// Risk confirmation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RiskConfig {
    /// Whether to confirm high-risk operations
    pub confirm_high_risk: bool,
    /// Whether to confirm medium-risk operations
    pub confirm_medium_risk: bool,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            confirm_high_risk: true,
            confirm_medium_risk: false,
        }
    }
}

/// Ignore configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct IgnoreConfig {
    /// Paths to ignore during scanning
    pub paths: Vec<PathBuf>,
}


impl Config {
    /// Load configuration from a TOML file
    pub fn load(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| crate::Error::Config(e.to_string()))?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from a path string
    pub fn load_from(path: &str) -> crate::Result<Self> {
        Self::load(std::path::Path::new(path))
    }

    /// Load configuration from the default location or create default
    pub fn load_or_default() -> Self {
        // Try to load from ~/.config/cleanmymac-rs/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("cleanmymac-rs").join("config.toml");
            if config_path.exists() {
                if let Ok(config) = Self::load(&config_path) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Get the default configuration path
    pub fn default_path() -> crate::Result<std::path::PathBuf> {
        dirs::config_dir()
            .map(|p| p.join("cleanmymac-rs").join("config.toml"))
            .ok_or_else(|| crate::Error::Config("Could not determine config directory".to_string()))
    }

    /// Save configuration to a TOML file
    pub fn save(&self, path: &std::path::Path) -> crate::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::Error::Config(e.to_string()))?;
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::Error::Config(e.to_string()))?;
        }
        
        std::fs::write(path, content)
            .map_err(|e| crate::Error::Config(e.to_string()))?;
        
        Ok(())
    }

    /// Save configuration to a path
    pub fn save_to(&self, path: &std::path::Path) -> crate::Result<()> {
        self.save(path)
    }
}
