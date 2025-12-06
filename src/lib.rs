//! CleanMyMac-rs: A cross-platform system cleaner for macOS and Linux
//!
//! This crate provides functionality to scan and clean various caches and
//! temporary files from development tools and system applications.

pub mod cleaner;
pub mod config;
pub mod error;
pub mod rules;
pub mod scanner;
pub mod ui;

pub use error::{Error, Result};
