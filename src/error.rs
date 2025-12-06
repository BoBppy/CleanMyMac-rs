//! Error types for CleanMyMac-rs

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using our custom Error
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for CleanMyMac-rs
#[derive(Error, Debug)]
pub enum Error {
    /// File system operation failed
    #[error("File system error at {path}: {source}")]
    FileSystem {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to read directory
    #[error("Failed to read directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to delete file or directory
    #[error("Failed to delete {path}: {source}")]
    Delete {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to move to trash
    #[error("Failed to move {path} to trash: {message}")]
    Trash { path: PathBuf, message: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Rule not found
    #[error("Rule not found: {0}")]
    RuleNotFound(String),

    /// Operation cancelled by user
    #[error("Operation cancelled by user")]
    Cancelled,

    /// Permission denied
    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    /// Generic IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] toml::de::Error),

    /// Walk directory error
    #[error("Walk directory error: {0}")]
    WalkDir(#[from] walkdir::Error),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a file system error
    pub fn filesystem(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::FileSystem {
            path: path.into(),
            source,
        }
    }

    /// Create a delete error
    pub fn delete(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Delete {
            path: path.into(),
            source,
        }
    }

    /// Create a trash error
    pub fn trash(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::Trash {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied(path: impl Into<PathBuf>) -> Self {
        Self::PermissionDenied { path: path.into() }
    }
}
