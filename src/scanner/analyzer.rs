//! Storage analyzer for analyzing disk usage

use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Storage usage information
#[derive(Debug, Clone, Default)]
pub struct StorageInfo {
    /// Total size analyzed
    pub total_size: u64,
    /// Number of files
    pub file_count: usize,
    /// Number of directories
    pub dir_count: usize,
    /// Size by file extension
    pub by_extension: HashMap<String, u64>,
    /// Largest files
    pub largest_files: Vec<(PathBuf, u64)>,
}

/// Storage analyzer
#[derive(Debug, Default)]
pub struct StorageAnalyzer {
    /// Maximum depth to analyze
    max_depth: Option<usize>,
    /// Number of largest files to track
    top_n: usize,
}

impl StorageAnalyzer {
    /// Create a new storage analyzer
    pub fn new() -> Self {
        Self {
            max_depth: None,
            top_n: 10,
        }
    }

    /// Set the maximum depth for analysis
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set the number of largest files to track
    pub fn with_top_n(mut self, n: usize) -> Self {
        self.top_n = n;
        self
    }

    /// Analyze a directory
    pub fn analyze(&self, path: &PathBuf) -> anyhow::Result<StorageInfo> {
        let mut info = StorageInfo::default();
        let mut largest: Vec<(PathBuf, u64)> = Vec::with_capacity(self.top_n + 1);

        let walker = if let Some(depth) = self.max_depth {
            WalkDir::new(path).max_depth(depth)
        } else {
            WalkDir::new(path)
        };

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.path();

            if let Ok(metadata) = entry_path.metadata() {
                if metadata.is_file() {
                    let size = metadata.len();
                    info.total_size += size;
                    info.file_count += 1;

                    // Track by extension
                    if let Some(ext) = entry_path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        *info.by_extension.entry(ext_str).or_insert(0) += size;
                    }

                    // Track largest files
                    largest.push((entry_path.to_path_buf(), size));
                    largest.sort_by(|a, b| b.1.cmp(&a.1));
                    largest.truncate(self.top_n);
                } else if metadata.is_dir() {
                    info.dir_count += 1;
                }
            }
        }

        info.largest_files = largest;
        Ok(info)
    }

    /// Analyze multiple directories
    pub fn analyze_multiple(&self, paths: &[PathBuf]) -> anyhow::Result<StorageInfo> {
        let mut combined = StorageInfo::default();

        for path in paths {
            if path.exists() {
                let info = self.analyze(path)?;
                combined.total_size += info.total_size;
                combined.file_count += info.file_count;
                combined.dir_count += info.dir_count;

                for (ext, size) in info.by_extension {
                    *combined.by_extension.entry(ext).or_insert(0) += size;
                }

                combined.largest_files.extend(info.largest_files);
            }
        }

        // Keep only top N largest files
        combined.largest_files.sort_by(|a, b| b.1.cmp(&a.1));
        combined.largest_files.truncate(self.top_n);

        Ok(combined)
    }
}

/// Format bytes to human-readable string
pub fn format_bytes(bytes: u64) -> String {
    bytesize::ByteSize::b(bytes).to_string()
}
