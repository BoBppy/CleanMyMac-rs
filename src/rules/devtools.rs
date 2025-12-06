//! Cross-platform development tools cleanup rules

use super::{Category, CleanItem, CleanResult, CleanRule, RiskLevel};
use std::path::PathBuf;
use walkdir::WalkDir;

/// Get all development tools rules
pub fn get_devtools_rules() -> Vec<Box<dyn CleanRule>> {
    vec![
        // Node.js
        Box::new(NpmCacheRule),
        Box::new(YarnCacheRule),
        Box::new(PnpmCacheRule),
        Box::new(NvmCacheRule),
        Box::new(BunCacheRule),
        Box::new(DenoCacheRule),
        // Python
        Box::new(PipCacheRule),
        Box::new(UvCacheRule),
        Box::new(CondaCacheRule),
        // Rust
        Box::new(CargoCacheRule),
        Box::new(CargoTargetRule),
        Box::new(RustupCacheRule),
        // Go
        Box::new(GoCacheRule),
        // Java
        Box::new(GradleCacheRule),
        Box::new(MavenCacheRule),
        // Android
        Box::new(AndroidCacheRule),
        // Docker
        Box::new(DockerCacheRule),
        // IDE & Editors
        Box::new(VSCodeCacheRule),
        Box::new(CursorCacheRule),
        Box::new(JetBrainsCacheRule),
        // Mobile
        Box::new(FlutterCacheRule),
        Box::new(DartPubCacheRule),
        // Ruby
        Box::new(RubyCacheRule),
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

// ============ Node.js Rules ============

/// npm cache rule
pub struct NpmCacheRule;

impl CleanRule for NpmCacheRule {
    fn name(&self) -> &str {
        "npm Cache"
    }

    fn category(&self) -> Category {
        Category::NodeJs
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "npm package download cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".npm/_cacache"));
            paths.push(home.join(".npm/_logs"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 0 {
                    let desc = if path.to_string_lossy().contains("_logs") {
                        "npm logs"
                    } else {
                        "npm download cache"
                    };
                    items.push(CleanItem::new(
                        path,
                        size,
                        desc,
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

/// yarn cache rule
pub struct YarnCacheRule;

impl CleanRule for YarnCacheRule {
    fn name(&self) -> &str {
        "Yarn Cache"
    }

    fn category(&self) -> Category {
        Category::NodeJs
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Yarn package cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".yarn/cache"));
            paths.push(home.join(".cache/yarn"));
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
                        "Yarn package cache",
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

/// pnpm cache rule
pub struct PnpmCacheRule;

impl CleanRule for PnpmCacheRule {
    fn name(&self) -> &str {
        "pnpm Store"
    }

    fn category(&self) -> Category {
        Category::NodeJs
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "pnpm content-addressable store"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".pnpm-store"));
            paths.push(home.join(".local/share/pnpm/store"));
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
                        "pnpm content store (shared across projects)",
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

// ============ Python Rules ============

/// pip cache rule
pub struct PipCacheRule;

impl CleanRule for PipCacheRule {
    fn name(&self) -> &str {
        "pip Cache"
    }

    fn category(&self) -> Category {
        Category::Python
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "pip package download cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(cache) = dirs::cache_dir() {
            paths.push(cache.join("pip"));
        }
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".cache/pip"));
            // macOS location
            paths.push(home.join("Library/Caches/pip"));
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
                        "pip download cache",
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

/// uv cache rule
pub struct UvCacheRule;

impl CleanRule for UvCacheRule {
    fn name(&self) -> &str {
        "uv Cache"
    }

    fn category(&self) -> Category {
        Category::Python
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "uv package manager cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(cache) = dirs::cache_dir() {
            paths.push(cache.join("uv"));
        }
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".cache/uv"));
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
                        "uv package cache",
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

/// Conda cache rule
pub struct CondaCacheRule;

impl CleanRule for CondaCacheRule {
    fn name(&self) -> &str {
        "Conda Package Cache"
    }

    fn category(&self) -> Category {
        Category::Python
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Conda/Miniconda package cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("anaconda3/pkgs"));
            paths.push(home.join("miniconda3/pkgs"));
            paths.push(home.join("miniforge3/pkgs"));
            paths.push(home.join(".conda/pkgs"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 100 * 1024 * 1024 {
                    // > 100MB
                    items.push(CleanItem::new(
                        path,
                        size,
                        "Conda package cache",
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

// ============ Rust Rules ============

/// Cargo cache rule
pub struct CargoCacheRule;

impl CleanRule for CargoCacheRule {
    fn name(&self) -> &str {
        "Cargo Registry Cache"
    }

    fn category(&self) -> Category {
        Category::Rust
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Cargo registry and git cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".cargo/registry/cache"));
            paths.push(home.join(".cargo/git/checkouts"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 0 {
                    let desc = if path.to_string_lossy().contains("git") {
                        "Cargo git checkouts"
                    } else {
                        "Cargo registry cache"
                    };
                    items.push(CleanItem::new(
                        path,
                        size,
                        desc,
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

/// Cargo target directories rule
pub struct CargoTargetRule;

impl CleanRule for CargoTargetRule {
    fn name(&self) -> &str {
        "Rust Build Artifacts"
    }

    fn category(&self) -> Category {
        Category::Rust
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Rust project target directories (build artifacts)"
    }

    fn is_applicable(&self) -> bool {
        true // Always applicable, will scan common locations
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        // Will scan home directory for Rust projects
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home);
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        
        // Common project locations
        let search_dirs = if let Some(home) = dirs::home_dir() {
            vec![
                home.join("Projects"),
                home.join("projects"),
                home.join("Code"),
                home.join("code"),
                home.join("Development"),
                home.join("dev"),
                home.join("src"),
            ]
        } else {
            vec![]
        };

        for search_dir in search_dirs {
            if search_dir.exists() {
                // Look for target directories
                for entry in WalkDir::new(&search_dir)
                    .max_depth(4)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = entry.path();
                    if path.is_dir() && path.file_name().map(|n| n == "target").unwrap_or(false) {
                        // Check if this is a Cargo project
                        let cargo_toml = path.parent().map(|p| p.join("Cargo.toml"));
                        if cargo_toml.map(|p| p.exists()).unwrap_or(false) {
                            let size = dir_size(path);
                            if size > 50 * 1024 * 1024 {
                                // > 50MB
                                let project_name = path
                                    .parent()
                                    .and_then(|p| p.file_name())
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "unknown".to_string());
                                items.push(CleanItem::new(
                                    path.to_path_buf(),
                                    size,
                                    format!("Rust build: {}", project_name),
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

// ============ Go Rules ============

/// Go module cache rule
pub struct GoCacheRule;

impl CleanRule for GoCacheRule {
    fn name(&self) -> &str {
        "Go Module Cache"
    }

    fn category(&self) -> Category {
        Category::Go
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Go module download cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("go/pkg/mod/cache"));
        }
        // Check GOPATH if set
        if let Ok(gopath) = std::env::var("GOPATH") {
            paths.push(PathBuf::from(gopath).join("pkg/mod/cache"));
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
                        "Go module cache",
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

// ============ Java Rules ============

/// Gradle cache rule
pub struct GradleCacheRule;

impl CleanRule for GradleCacheRule {
    fn name(&self) -> &str {
        "Gradle Cache"
    }

    fn category(&self) -> Category {
        Category::Java
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Gradle build cache and dependencies"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".gradle/caches"));
            paths.push(home.join(".gradle/wrapper/dists"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 0 {
                    let desc = if path.to_string_lossy().contains("wrapper") {
                        "Gradle wrapper distributions"
                    } else {
                        "Gradle cache"
                    };
                    items.push(CleanItem::new(
                        path,
                        size,
                        desc,
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

/// Maven local repository rule
pub struct MavenCacheRule;

impl CleanRule for MavenCacheRule {
    fn name(&self) -> &str {
        "Maven Local Repository"
    }

    fn category(&self) -> Category {
        Category::Java
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Maven local repository cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".m2/repository"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 100 * 1024 * 1024 {
                    // > 100MB
                    items.push(CleanItem::new(
                        path,
                        size,
                        "Maven local repository",
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

// ============ Android Rules ============

/// Android SDK cache rule
pub struct AndroidCacheRule;

impl CleanRule for AndroidCacheRule {
    fn name(&self) -> &str {
        "Android SDK Cache"
    }

    fn category(&self) -> Category {
        Category::Android
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Android SDK and AVD cache files"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".android/cache"));
            paths.push(home.join(".android/build-cache"));
            // macOS location
            paths.push(home.join("Library/Android/sdk/.downloadIntermediates"));
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
                        "Android SDK cache",
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

// ============ Docker Rules ============

/// Docker cache rule
pub struct DockerCacheRule;

impl CleanRule for DockerCacheRule {
    fn name(&self) -> &str {
        "Docker Cache"
    }

    fn category(&self) -> Category {
        Category::Docker
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Docker build cache and unused data"
    }

    fn is_applicable(&self) -> bool {
        // Check if docker command exists
        std::process::Command::new("docker")
            .arg("--version")
            .output()
            .is_ok()
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        // Docker manages its own storage, we'll use docker system df
        vec![]
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        // Try to get docker system info
        let output = std::process::Command::new("docker")
            .args(["system", "df", "--format", "{{.Reclaimable}}"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut total_reclaimable = 0u64;
                
                for line in stdout.lines() {
                    // Parse sizes like "1.5GB", "500MB"
                    let trimmed = line.trim().to_uppercase();
                    if let Some(size) = parse_size(&trimmed) {
                        total_reclaimable += size;
                    }
                }

                if total_reclaimable > 100 * 1024 * 1024 {
                    // > 100MB
                    Ok(vec![CleanItem::new(
                        PathBuf::from("/var/lib/docker"),
                        total_reclaimable,
                        "Docker reclaimable space (run 'docker system prune')",
                        RiskLevel::Medium,
                        Category::Docker,
                    )])
                } else {
                    Ok(vec![])
                }
            }
            _ => Ok(vec![]),
        }
    }

    fn clean(&self, _items: &[CleanItem], _to_trash: bool) -> anyhow::Result<CleanResult> {
        // Execute docker system prune
        let output = std::process::Command::new("docker")
            .args(["system", "prune", "-f"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Try to parse reclaimed space from output
                let bytes_freed = stdout
                    .lines()
                    .find(|l| l.contains("reclaimed"))
                    .and_then(|l| {
                        l.split_whitespace()
                            .find(|s| s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false))
                            .and_then(|s| parse_size(&s.to_uppercase()))
                    })
                    .unwrap_or(0);

                Ok(CleanResult {
                    cleaned_count: 1,
                    bytes_freed,
                    failed: vec![],
                    cancelled: false,
                })
            }
            Ok(output) => Ok(CleanResult {
                cleaned_count: 0,
                bytes_freed: 0,
                failed: vec![(
                    PathBuf::from("docker"),
                    String::from_utf8_lossy(&output.stderr).to_string(),
                )],
                cancelled: false,
            }),
            Err(e) => Ok(CleanResult {
                cleaned_count: 0,
                bytes_freed: 0,
                failed: vec![(PathBuf::from("docker"), e.to_string())],
                cancelled: false,
            }),
        }
    }
}

/// Parse size strings like "1.5GB", "500MB", "1024KB"
fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (num_part, unit) = if s.ends_with("GB") {
        (s.trim_end_matches("GB"), 1024 * 1024 * 1024)
    } else if s.ends_with("MB") {
        (s.trim_end_matches("MB"), 1024 * 1024)
    } else if s.ends_with("KB") {
        (s.trim_end_matches("KB"), 1024)
    } else if s.ends_with("B") {
        (s.trim_end_matches("B"), 1)
    } else {
        return None;
    };

    num_part.trim().parse::<f64>().ok().map(|n| (n * unit as f64) as u64)
}

// ============ IDE & Editor Rules ============

/// VSCode cache rule
pub struct VSCodeCacheRule;

impl CleanRule for VSCodeCacheRule {
    fn name(&self) -> &str {
        "VS Code Cache"
    }

    fn category(&self) -> Category {
        Category::Other("IDE".to_string())
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "VS Code extensions cache and logs"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".vscode/extensions"));
            paths.push(home.join("Library/Application Support/Code/Cache"));
            paths.push(home.join("Library/Application Support/Code/CachedData"));
            paths.push(home.join("Library/Application Support/Code/CachedExtensions"));
            paths.push(home.join("Library/Application Support/Code/logs"));
            paths.push(home.join("Library/Caches/com.microsoft.VSCode"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 10 * 1024 * 1024 {
                    let is_extensions = path.to_string_lossy().contains("extensions");
                    let desc = if is_extensions {
                        "VS Code extensions (consider cleaning unused)"
                    } else if path.to_string_lossy().contains("logs") {
                        "VS Code logs"
                    } else {
                        "VS Code cache"
                    };
                    items.push(CleanItem::new(
                        path,
                        size,
                        desc,
                        if is_extensions {
                            RiskLevel::Medium
                        } else {
                            self.risk_level()
                        },
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

/// Cursor IDE cache rule
pub struct CursorCacheRule;

impl CleanRule for CursorCacheRule {
    fn name(&self) -> &str {
        "Cursor Cache"
    }

    fn category(&self) -> Category {
        Category::Other("IDE".to_string())
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Cursor AI IDE cache and logs"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".cursor/extensions"));
            paths.push(home.join("Library/Application Support/Cursor/Cache"));
            paths.push(home.join("Library/Application Support/Cursor/CachedData"));
            paths.push(home.join("Library/Application Support/Cursor/logs"));
            paths.push(home.join("Library/Caches/com.todesktop.230313mzl4w4u92"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 10 * 1024 * 1024 {
                    let is_extensions = path.to_string_lossy().contains("extensions");
                    let desc = if is_extensions {
                        "Cursor extensions"
                    } else if path.to_string_lossy().contains("logs") {
                        "Cursor logs"
                    } else {
                        "Cursor cache"
                    };
                    items.push(CleanItem::new(
                        path,
                        size,
                        desc,
                        if is_extensions {
                            RiskLevel::Medium
                        } else {
                            self.risk_level()
                        },
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

/// JetBrains IDEs cache rule
pub struct JetBrainsCacheRule;

impl CleanRule for JetBrainsCacheRule {
    fn name(&self) -> &str {
        "JetBrains Cache"
    }

    fn category(&self) -> Category {
        Category::Other("IDE".to_string())
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "JetBrains IDEs cache (IntelliJ, WebStorm, PyCharm, etc.)"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            // Look for JetBrains cache directories
            let cache_base = home.join("Library/Caches/JetBrains");
            if cache_base.exists() {
                paths.push(cache_base);
            }
            // Also check for individual IDE caches
            let ides = ["IntelliJIdea", "WebStorm", "PyCharm", "CLion", "GoLand", "RustRover", "DataGrip"];
            for ide in &ides {
                let pattern = home.join(format!("Library/Caches/{}", ide));
                if pattern.exists() {
                    paths.push(pattern);
                }
            }
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 50 * 1024 * 1024 {
                    items.push(CleanItem::new(
                        path,
                        size,
                        "JetBrains IDE cache",
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

// ============ Mobile Development Rules ============

/// Flutter cache rule
pub struct FlutterCacheRule;

impl CleanRule for FlutterCacheRule {
    fn name(&self) -> &str {
        "Flutter Cache"
    }

    fn category(&self) -> Category {
        Category::Other("Mobile".to_string())
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Flutter SDK cache and artifacts"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("flutter/bin/cache"));
            paths.push(home.join(".flutter"));
            paths.push(home.join("development/flutter/bin/cache"));
        }
        // Check FLUTTER_ROOT if set
        if let Ok(flutter_root) = std::env::var("FLUTTER_ROOT") {
            paths.push(PathBuf::from(flutter_root).join("bin/cache"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 100 * 1024 * 1024 {
                    items.push(CleanItem::new(
                        path,
                        size,
                        "Flutter SDK cache",
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

/// Dart pub cache rule
pub struct DartPubCacheRule;

impl CleanRule for DartPubCacheRule {
    fn name(&self) -> &str {
        "Dart Pub Cache"
    }

    fn category(&self) -> Category {
        Category::Other("Mobile".to_string())
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Dart package manager cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".pub-cache"));
        }
        // Check PUB_CACHE if set
        if let Ok(pub_cache) = std::env::var("PUB_CACHE") {
            paths.push(PathBuf::from(pub_cache));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 50 * 1024 * 1024 {
                    items.push(CleanItem::new(
                        path,
                        size,
                        "Dart pub cache",
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

// ============ Additional Node.js Tools ============

/// nvm cache rule
pub struct NvmCacheRule;

impl CleanRule for NvmCacheRule {
    fn name(&self) -> &str {
        "nvm Cache"
    }

    fn category(&self) -> Category {
        Category::NodeJs
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Node Version Manager installed versions"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".nvm/versions"));
            paths.push(home.join(".nvm/.cache"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 100 * 1024 * 1024 {
                    let desc = if path.to_string_lossy().contains(".cache") {
                        "nvm download cache"
                    } else {
                        "nvm Node.js versions (keep versions you use)"
                    };
                    items.push(CleanItem::new(
                        path,
                        size,
                        desc,
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

/// Bun cache rule
pub struct BunCacheRule;

impl CleanRule for BunCacheRule {
    fn name(&self) -> &str {
        "Bun Cache"
    }

    fn category(&self) -> Category {
        Category::NodeJs
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Bun package manager cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".bun/install/cache"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 50 * 1024 * 1024 {
                    items.push(CleanItem::new(
                        path,
                        size,
                        "Bun package cache",
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

/// Deno cache rule
pub struct DenoCacheRule;

impl CleanRule for DenoCacheRule {
    fn name(&self) -> &str {
        "Deno Cache"
    }

    fn category(&self) -> Category {
        Category::NodeJs
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Deno runtime cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(cache) = dirs::cache_dir() {
            paths.push(cache.join("deno"));
        }
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".deno"));
            paths.push(home.join("Library/Caches/deno"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 50 * 1024 * 1024 {
                    items.push(CleanItem::new(
                        path,
                        size,
                        "Deno cache",
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

// ============ Rustup Rule ============

/// Rustup cache rule
pub struct RustupCacheRule;

impl CleanRule for RustupCacheRule {
    fn name(&self) -> &str {
        "Rustup Toolchains"
    }

    fn category(&self) -> Category {
        Category::Rust
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Rustup installed toolchains and components"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".rustup/toolchains"));
            paths.push(home.join(".rustup/downloads"));
            paths.push(home.join(".rustup/tmp"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 500 * 1024 * 1024 {
                    let desc = if path.to_string_lossy().contains("toolchains") {
                        "Rustup toolchains (keep versions you use)"
                    } else if path.to_string_lossy().contains("downloads") {
                        "Rustup downloads cache"
                    } else {
                        "Rustup temporary files"
                    };
                    items.push(CleanItem::new(
                        path,
                        size,
                        desc,
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

// ============ Ruby Rule ============

/// Ruby cache rule
pub struct RubyCacheRule;

impl CleanRule for RubyCacheRule {
    fn name(&self) -> &str {
        "Ruby Gems Cache"
    }

    fn category(&self) -> Category {
        Category::Other("Ruby".to_string())
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn description(&self) -> &str {
        "Ruby gems and bundle cache"
    }

    fn is_applicable(&self) -> bool {
        self.scan_paths().iter().any(|p| p.exists())
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".gem"));
            paths.push(home.join(".bundle/cache"));
            paths.push(home.join(".rbenv/versions"));
        }
        paths
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();
        for path in self.scan_paths() {
            if path.exists() {
                let size = dir_size(&path);
                if size > 100 * 1024 * 1024 {
                    let desc = if path.to_string_lossy().contains("rbenv") {
                        "rbenv Ruby versions"
                    } else if path.to_string_lossy().contains("bundle") {
                        "Bundler cache"
                    } else {
                        "Ruby gems cache"
                    };
                    items.push(CleanItem::new(
                        path,
                        size,
                        desc,
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

