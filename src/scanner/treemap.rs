//! High-performance Treemap implementation using Squarified algorithm
//!
//! This module provides a treemap visualization for storage analysis,
//! similar to SpaceSniffer/WinDirStat. Uses the Squarified Treemap algorithm
//! for optimal visual aspect ratios.

use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A node in the directory tree
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Path to this node
    pub path: PathBuf,
    /// Name of this node
    pub name: String,
    /// Size in bytes
    pub size: u64,
    /// Children nodes (for directories)
    pub children: Vec<TreeNode>,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Depth in the tree
    pub depth: usize,
}

impl TreeNode {
    /// Create a new tree node
    pub fn new(path: PathBuf, name: String, size: u64, is_dir: bool, depth: usize) -> Self {
        Self {
            path,
            name,
            size,
            children: Vec::new(),
            is_dir,
            depth,
        }
    }

    /// Get percentage of parent size
    pub fn percentage(&self, parent_size: u64) -> f64 {
        if parent_size == 0 {
            0.0
        } else {
            (self.size as f64 / parent_size as f64) * 100.0
        }
    }
}

/// Rectangle for treemap layout
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Get the shorter side
    pub fn shorter_side(&self) -> f64 {
        self.width.min(self.height)
    }

    /// Check if width is shorter
    pub fn is_horizontal(&self) -> bool {
        self.width < self.height
    }
}

/// A positioned treemap item for rendering
#[derive(Debug, Clone)]
pub struct TreemapItem {
    pub node: TreeNode,
    pub rect: Rect,
    pub color_index: usize,
}

/// High-performance treemap builder
pub struct TreemapBuilder {
    /// Maximum depth to scan
    max_depth: usize,
    /// Minimum size to include (bytes)
    min_size: u64,
    /// Use parallel scanning
    parallel: bool,
}

impl Default for TreemapBuilder {
    fn default() -> Self {
        Self {
            max_depth: 5,
            min_size: 1024 * 1024, // 1MB minimum
            parallel: true,
        }
    }
}

impl TreemapBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn min_size(mut self, size: u64) -> Self {
        self.min_size = size;
        self
    }

    pub fn parallel(mut self, enabled: bool) -> Self {
        self.parallel = enabled;
        self
    }

    /// Build a tree from a directory path
    pub fn build_tree(&self, root: &Path) -> anyhow::Result<TreeNode> {
        self.build_tree_recursive(root, 0)
    }

    fn build_tree_recursive(&self, path: &Path, depth: usize) -> anyhow::Result<TreeNode> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        if !path.is_dir() {
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            return Ok(TreeNode::new(path.to_path_buf(), name, size, false, depth));
        }

        // Read directory entries
        let entries: Vec<_> = std::fs::read_dir(path)?.filter_map(|e| e.ok()).collect();

        // Process children (parallel if enabled and depth allows)
        let children: Vec<TreeNode> = if self.parallel && depth < 2 {
            entries
                .par_iter()
                .filter_map(|entry| {
                    let child_path = entry.path();
                    if depth < self.max_depth {
                        self.build_tree_recursive(&child_path, depth + 1).ok()
                    } else if child_path.is_dir() {
                        // For deep directories, just calculate total size
                        let size = self.calculate_dir_size(&child_path);
                        let name = child_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        Some(TreeNode::new(child_path, name, size, true, depth + 1))
                    } else {
                        let size = child_path.metadata().map(|m| m.len()).unwrap_or(0);
                        let name = child_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        Some(TreeNode::new(child_path, name, size, false, depth + 1))
                    }
                })
                .filter(|node| node.size >= self.min_size)
                .collect()
        } else {
            entries
                .iter()
                .filter_map(|entry| {
                    let child_path = entry.path();
                    if depth < self.max_depth {
                        self.build_tree_recursive(&child_path, depth + 1).ok()
                    } else if child_path.is_dir() {
                        let size = self.calculate_dir_size(&child_path);
                        let name = child_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        Some(TreeNode::new(child_path, name, size, true, depth + 1))
                    } else {
                        let size = child_path.metadata().map(|m| m.len()).unwrap_or(0);
                        let name = child_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        Some(TreeNode::new(child_path, name, size, false, depth + 1))
                    }
                })
                .filter(|node| node.size >= self.min_size)
                .collect()
        };

        let total_size: u64 = children.iter().map(|c| c.size).sum();

        let mut node = TreeNode::new(path.to_path_buf(), name, total_size, true, depth);
        node.children = children;

        // Sort children by size descending
        node.children.sort_by(|a, b| b.size.cmp(&a.size));

        Ok(node)
    }

    /// Calculate directory size using parallel walk
    fn calculate_dir_size(&self, path: &Path) -> u64 {
        WalkDir::new(path)
            .into_iter()
            .par_bridge()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    }
}

/// Squarified treemap layout algorithm
pub struct SquarifiedLayout;

impl SquarifiedLayout {
    /// Layout a tree into rectangles using the Squarified algorithm
    pub fn layout(root: &TreeNode, bounds: Rect) -> Vec<TreemapItem> {
        let mut items = Vec::new();
        Self::layout_recursive(root, bounds, 0, &mut items);
        items
    }

    fn layout_recursive(
        node: &TreeNode,
        bounds: Rect,
        color_index: usize,
        items: &mut Vec<TreemapItem>,
    ) {
        if node.children.is_empty() {
            items.push(TreemapItem {
                node: node.clone(),
                rect: bounds,
                color_index,
            });
            return;
        }

        // Get sizes for children
        let sizes: Vec<f64> = node.children.iter().map(|c| c.size as f64).collect();
        let total: f64 = sizes.iter().sum();

        if total == 0.0 {
            return;
        }

        // Calculate rectangles using squarified algorithm
        let rects = Self::squarify(&sizes, bounds, total);

        // Recursively layout children
        for (i, (child, rect)) in node.children.iter().zip(rects.iter()).enumerate() {
            if child.children.is_empty() {
                items.push(TreemapItem {
                    node: child.clone(),
                    rect: *rect,
                    color_index: (color_index + i) % 12,
                });
            } else {
                Self::layout_recursive(child, *rect, (color_index + i) % 12, items);
            }
        }
    }

    /// Squarified treemap algorithm
    fn squarify(sizes: &[f64], bounds: Rect, total: f64) -> Vec<Rect> {
        if sizes.is_empty() {
            return Vec::new();
        }

        let area = bounds.width * bounds.height;
        let normalized: Vec<f64> = sizes.iter().map(|s| s / total * area).collect();

        let mut rects = Vec::with_capacity(sizes.len());
        let mut remaining = bounds;
        let mut start = 0;

        while start < normalized.len() {
            let (row, end) = Self::find_best_row(&normalized[start..], remaining);
            let row_rects = Self::layout_row(&row, remaining);

            // Update remaining area
            let row_area: f64 = row.iter().sum();
            if remaining.is_horizontal() {
                let width = row_area / remaining.height;
                remaining = Rect::new(
                    remaining.x + width,
                    remaining.y,
                    remaining.width - width,
                    remaining.height,
                );
            } else {
                let height = row_area / remaining.width;
                remaining = Rect::new(
                    remaining.x,
                    remaining.y + height,
                    remaining.width,
                    remaining.height - height,
                );
            }

            rects.extend(row_rects);
            start += end;
        }

        rects
    }

    /// Find the best row of items that minimizes aspect ratio
    fn find_best_row(sizes: &[f64], bounds: Rect) -> (Vec<f64>, usize) {
        if sizes.is_empty() {
            return (Vec::new(), 0);
        }

        let side = bounds.shorter_side();
        let mut row = vec![sizes[0]];
        let mut best_ratio = Self::worst_ratio(&row, side);

        for (i, &size) in sizes.iter().enumerate().skip(1) {
            let mut test_row = row.clone();
            test_row.push(size);
            let ratio = Self::worst_ratio(&test_row, side);

            if ratio > best_ratio {
                // Adding this item makes it worse, stop here
                return (row, i);
            }

            row = test_row;
            best_ratio = ratio;
        }

        (row, sizes.len())
    }

    /// Calculate the worst aspect ratio in a row
    fn worst_ratio(row: &[f64], side: f64) -> f64 {
        if row.is_empty() || side == 0.0 {
            return f64::MAX;
        }

        let sum: f64 = row.iter().sum();
        let side_sq = side * side;

        row.iter()
            .map(|&r| {
                let ratio1 = (side_sq * r) / (sum * sum);
                let ratio2 = (sum * sum) / (side_sq * r);
                ratio1.max(ratio2)
            })
            .fold(0.0, f64::max)
    }

    /// Layout a row of items
    fn layout_row(row: &[f64], bounds: Rect) -> Vec<Rect> {
        if row.is_empty() {
            return Vec::new();
        }

        let sum: f64 = row.iter().sum();
        let mut rects = Vec::with_capacity(row.len());

        if bounds.is_horizontal() {
            // Lay out horizontally (split width)
            let width = sum / bounds.height;
            let mut y = bounds.y;

            for &size in row {
                let height = size / width;
                rects.push(Rect::new(bounds.x, y, width, height));
                y += height;
            }
        } else {
            // Lay out vertically (split height)
            let height = sum / bounds.width;
            let mut x = bounds.x;

            for &size in row {
                let width = size / height;
                rects.push(Rect::new(x, bounds.y, width, height));
                x += width;
            }
        }

        rects
    }
}

/// Extension analysis for the treemap
#[derive(Debug, Clone)]
pub struct ExtensionStats {
    pub extension: String,
    pub size: u64,
    pub count: usize,
    pub percentage: f64,
}

/// Analyze a tree for extension statistics
pub fn analyze_extensions(root: &TreeNode) -> Vec<ExtensionStats> {
    let mut stats: HashMap<String, (u64, usize)> = HashMap::new();
    collect_extensions(root, &mut stats);

    let total: u64 = stats.values().map(|(s, _)| s).sum();

    let mut result: Vec<ExtensionStats> = stats
        .into_iter()
        .map(|(ext, (size, count))| ExtensionStats {
            extension: ext,
            size,
            count,
            percentage: if total > 0 {
                (size as f64 / total as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect();

    result.sort_by(|a, b| b.size.cmp(&a.size));
    result
}

fn collect_extensions(node: &TreeNode, stats: &mut HashMap<String, (u64, usize)>) {
    if !node.is_dir {
        let ext = node
            .path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_else(|| "(no ext)".to_string());

        let entry = stats.entry(ext).or_insert((0, 0));
        entry.0 += node.size;
        entry.1 += 1;
    }

    for child in &node.children {
        collect_extensions(child, stats);
    }
}

/// Get top N largest items from a tree
pub fn get_largest_items(root: &TreeNode, n: usize) -> Vec<&TreeNode> {
    let mut items = Vec::new();
    collect_items(root, &mut items);
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items.into_iter().take(n).collect()
}

fn collect_items<'a>(node: &'a TreeNode, items: &mut Vec<&'a TreeNode>) {
    items.push(node);
    for child in &node.children {
        collect_items(child, items);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_shorter_side() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(rect.shorter_side(), 50.0);
    }

    #[test]
    fn test_squarify_single() {
        let sizes = vec![100.0];
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let rects = SquarifiedLayout::squarify(&sizes, bounds, 100.0);
        assert_eq!(rects.len(), 1);
    }
}
