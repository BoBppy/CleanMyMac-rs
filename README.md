# CleanMyMac-rs

A cross-platform system cleaner for macOS and Linux, built with Rust for performance and safety.

![Rust](https://img.shields.io/badge/rust-2024-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey.svg)

## Features

- 🚀 **High Performance** - Built with Rust, uses parallel scanning via rayon
- 🍎 **macOS Support** - Homebrew, Xcode, CocoaPods, iOS Simulators, App caches
- 🐧 **Linux Support** - APT, DNF, Pacman, Snap, Flatpak, systemd logs
- 🔧 **Dev Tools** - npm, yarn, pip, uv, Cargo, Go, Gradle, Maven, Docker
- 🔍 **Heuristic Detection** - Auto-discover large cache directories
- 🛡️ **Safe Cleaning** - Move to trash by default, high-risk confirmation prompts
- 📊 **Storage Analysis** - Analyze disk usage by file type

## Installation

### Download Pre-built Binaries

Download the latest nightly build from the [pre-release page](https://github.com/BoBppy/CleanMyMac-rs/releases/tag/nightly).

Available platforms:
- macOS (ARM64): `cleanmymac-rs-macos-aarch64.tar.gz`
- Linux (x86_64): `cleanmymac-rs-linux-x86_64.tar.gz`

### Build from Source

```bash
# Clone the repository
git clone https://github.com/BoBppy/CleanMyMac-rs.git
cd CleanMyMac-rs

# Build release version
cargo build --release

# Install (optional)
cargo install --path .
```

## Usage

```bash
# Scan for cleanable files
cleanmymac-rs scan

# Clean with confirmation
cleanmymac-rs clean

# Dry run (preview what would be deleted)
cleanmymac-rs clean --dry-run

# Clean specific categories
cleanmymac-rs clean --categories brew,npm,cargo

# Analyze storage usage
cleanmymac-rs analyze

# List available cleanup rules
cleanmymac-rs list --detailed

# Show help
cleanmymac-rs --help
```

## Cleanup Categories

| Category | Description |
|----------|-------------|
| System | User caches and logs |
| Brew | Homebrew package cache |
| Xcode | DerivedData, Archives, Device Support |
| NodeJs | npm, yarn, pnpm caches |
| Python | pip, uv, Conda caches |
| Rust | Cargo registry and build artifacts |
| Go | Go module cache |
| Java | Gradle and Maven caches |
| Docker | Docker system cache |
| Android | Android SDK cache |
| Heuristic | Auto-detected cache directories |

## Configuration

Initialize default configuration:
```bash
cleanmymac-rs config --init
```

Configuration file location: `~/.config/cleanmymac-rs/config.toml`

## Safety

- Files are moved to system trash by default
- High-risk operations require explicit confirmation
- Use `--dry-run` to preview changes
- Use `--permanent` only when you're sure

## Requirements

- Rust 1.85+ (2024 Edition)
- macOS 14+ or Linux

## License

MIT License - see [LICENSE](LICENSE) for details.
