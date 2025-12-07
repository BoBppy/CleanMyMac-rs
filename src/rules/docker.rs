//! Docker cleanup rules

use super::{Category, CleanItem, CleanResult, CleanRule, RiskLevel};
use std::path::PathBuf;
use std::process::Command;

/// Docker cleanup rule
pub struct DockerRule;

impl CleanRule for DockerRule {
    fn name(&self) -> &str {
        "Docker Cleanup"
    }

    fn category(&self) -> Category {
        Category::Docker
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Dangling images, stopped containers, and unused networks"
    }

    fn is_applicable(&self) -> bool {
        // Check if docker command exists and is running
        Command::new("docker")
            .arg("info")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn scan_paths(&self) -> Vec<PathBuf> {
        Vec::new() // Not path based
    }

    fn scan(&self) -> anyhow::Result<Vec<CleanItem>> {
        let mut items = Vec::new();

        // Check size of reclaimable space
        let output = Command::new("docker")
            .args(["system", "df", "--format", "{{.Type}}\t{{.Reclaimable}}"])
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let _total_size = 0;
            let _details = String::new();

            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    // unexpected format parsing is tricky with units,
                    // attempting to parse "1.2GB" or similar is complex without a library.
                    // For now, let's rely on a simpler check for count of objects.
                }
            }
        }

        // Alternative: Count dangling images
        let images_out = Command::new("docker")
            .args(["images", "-f", "dangling=true", "--format", "{{.Size}}"])
            .output()?;

        if images_out.status.success() {
            let stdout = String::from_utf8_lossy(&images_out.stdout);
            let mut size = 0;
            let mut count = 0;

            for line in stdout.lines() {
                // Parse size roughly (e.g. "100MB")
                // This is a bit fragile without a proper size parser.
                // Let's assume 0 size for safety if parsing fails, but count items.
                size += parse_docker_size(line);
                count += 1;
            }

            if count > 0 {
                items.push(CleanItem::new(
                    PathBuf::from("Docker Dangling Imagess"), // Virtual path
                    size,
                    format!("{} dangling images", count),
                    self.risk_level(),
                    self.category(),
                ));
            }
        }

        // Check stopped containers
        let containers_out = Command::new("docker")
            .args(["ps", "-a", "-f", "status=exited", "-q"])
            .output()?;

        if containers_out.status.success() {
            let count = String::from_utf8_lossy(&containers_out.stdout)
                .lines()
                .count();
            if count > 0 {
                items.push(CleanItem::new(
                    PathBuf::from("Docker Stopped Containers"),
                    0, // Hard to get exact reclaimable size easily
                    format!("{} stopped containers", count),
                    self.risk_level(),
                    self.category(),
                ));
            }
        }

        Ok(items)
    }

    fn clean(&self, items: &[CleanItem], _to_trash: bool) -> anyhow::Result<CleanResult> {
        let mut result = CleanResult::default();

        for item in items {
            let mut cmd = Command::new("docker");
            if item.path.to_string_lossy().contains("Images") {
                cmd.args(["image", "prune", "-f"]);
            } else if item.path.to_string_lossy().contains("Containers") {
                cmd.args(["container", "prune", "-f"]);
            } else {
                continue;
            }

            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        result.cleaned_count += 1;
                        result.bytes_freed += item.size;
                    } else {
                        result.failed.push((
                            item.path.clone(),
                            String::from_utf8_lossy(&output.stderr).to_string(),
                        ));
                    }
                }
                Err(e) => {
                    result.failed.push((item.path.clone(), e.to_string()));
                }
            }
        }

        Ok(result)
    }
}

fn parse_docker_size(size_str: &str) -> u64 {
    let s = size_str.trim();
    if s.is_empty() {
        return 0;
    }

    let bytes = if s.ends_with("GB") {
        s.trim_end_matches("GB").parse::<f64>().unwrap_or(0.0) * 1_000_000_000.0
    } else if s.ends_with("MB") {
        s.trim_end_matches("MB").parse::<f64>().unwrap_or(0.0) * 1_000_000.0
    } else if s.ends_with("KB") {
        s.trim_end_matches("KB").parse::<f64>().unwrap_or(0.0) * 1_000.0
    } else if s.ends_with("B") {
        s.trim_end_matches("B").parse::<f64>().unwrap_or(0.0)
    } else {
        0.0
    };

    bytes as u64
}
