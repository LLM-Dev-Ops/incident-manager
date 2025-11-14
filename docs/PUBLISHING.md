# Publishing Guide for llm-incident-manager

This guide covers publishing the `llm-incident-manager` crate to crates.io.

## Overview

**Crate Name**: `llm-incident-manager`
**Version**: 1.0.0
**License**: MIT OR Apache-2.0
**Repository**: https://github.com/globalbusinessadvisors/llm-incident-manager

This crate includes:
- **Library**: `llm-incident-manager` - Core incident management library
- **Binary 1**: `llm-incident-manager` - Main server application
- **Binary 2**: `llm-im-cli` - CLI tool for incident management

## Prerequisites

### 1. Rust Installation

If Rust is not installed:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Load environment
source "$HOME/.cargo/env"

# Verify
rustc --version
cargo --version
```

### 2. Crates.io Account

1. Create an account at https://crates.io
2. Generate an API token:
   - Go to https://crates.io/me
   - Navigate to "Account Settings" → "API Tokens"
   - Click "New Token"
   - Copy the token (you won't see it again!)

### 3. Authentication

**Option A: Interactive Login**
```bash
cargo login
# Paste your token when prompted
```

**Option B: Environment Variable (CI/CD)**
```bash
export CARGO_REGISTRY_TOKEN="your-token-here"
```

## Manual Publishing

### Method 1: Using the Publish Script (Recommended)

```bash
# Dry-run (test without publishing)
./scripts/publish.sh --dry-run

# Publish with token from environment
export CARGO_REGISTRY_TOKEN="your-token"
./scripts/publish.sh

# Publish with token as argument
./scripts/publish.sh --token "your-token"
```

### Method 2: Using Makefile

```bash
# Dry-run
make publish-dry-run

# Publish (requires CARGO_REGISTRY_TOKEN env var)
export CARGO_REGISTRY_TOKEN="your-token"
make publish
```

### Method 3: Direct Cargo Commands

```bash
# 1. Run tests
cargo test --all-features

# 2. Dry-run
cargo publish --dry-run

# 3. Publish
cargo login  # If not already logged in
cargo publish
```

## Automated Publishing (GitHub Actions)

The repository includes a GitHub Actions workflow (`.github/workflows/publish-crate.yml`) that automatically publishes to crates.io when you push a version tag.

### Setup

1. Add your crates.io token to GitHub secrets:
   - Go to your GitHub repository settings
   - Navigate to "Secrets and variables" → "Actions"
   - Add a new secret named `CRATES_TOKEN`
   - Paste your crates.io API token

2. Push a version tag:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

The workflow will:
- Install Rust
- Run tests
- Verify the package
- Publish to crates.io
- Create a GitHub release

### Manual Trigger

You can also manually trigger the workflow:
- Go to "Actions" tab in GitHub
- Select "Publish to crates.io"
- Click "Run workflow"

## After Publishing

### Verify Publication

1. Check crates.io:
   ```
   https://crates.io/crates/llm-incident-manager
   ```

2. Test installation:
   ```bash
   # Install the server
   cargo install llm-incident-manager

   # Install just the CLI
   cargo install llm-incident-manager --bin llm-im-cli
   ```

### Update Documentation

After first publication:
- Documentation will be available at https://docs.rs/llm-incident-manager
- Update README badges if needed

## Publishing Checklist

Before publishing a new version:

- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGELOG.md` (if exists)
- [ ] Run `cargo test --all-features`
- [ ] Run `cargo clippy -- -D warnings`
- [ ] Run `cargo fmt --check`
- [ ] Run `cargo publish --dry-run`
- [ ] Commit all changes
- [ ] Tag the release: `git tag vX.Y.Z`
- [ ] Push tag: `git push origin vX.Y.Z`
- [ ] Publish: `cargo publish`

## Version Updates

To publish a new version:

1. Update version in `Cargo.toml`:
   ```toml
   [package]
   version = "1.0.1"  # Increment according to semver
   ```

2. Commit and tag:
   ```bash
   git add Cargo.toml
   git commit -m "chore: bump version to 1.0.1"
   git tag v1.0.1
   git push origin main
   git push origin v1.0.1
   ```

3. Publish:
   ```bash
   cargo publish
   ```

## Troubleshooting

### "no token found"
```bash
# Run cargo login first
cargo login
# Or set environment variable
export CARGO_REGISTRY_TOKEN="your-token"
```

### "crate already exists"
- You cannot republish the same version
- Increment the version in `Cargo.toml`

### "file size too large"
- The `exclude` field in `Cargo.toml` already excludes large files
- Check package size: `cargo publish --dry-run`

### "missing license file"
- Both `LICENSE-MIT` and `LICENSE-APACHE` are included
- Cargo.toml specifies: `license = "MIT OR Apache-2.0"`

## Package Contents

Current package size: **380.3 KiB** (compressed)

Included:
- Source code (`src/`)
- Build scripts (`build.rs`)
- Proto files (`proto/`)
- Example configurations (`config/`)
- Examples (`examples/grpc/`)
- README and licenses

Excluded (to reduce size):
- Documentation (`docs/`)
- Tests (`tests/`)
- Benchmarks (`benches/`)
- Node modules (`node_modules/`)
- IDE configs (`.vscode/`, `.idea/`)
- Logs and temporary files

## Support

For issues with publishing:
- Check the [Cargo documentation](https://doc.rust-lang.org/cargo/reference/publishing.html)
- Visit the [crates.io help](https://crates.io/policies)
- Review the [Rust package publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html)

## Ownership Management

### Add Co-Owners

```bash
# Add a co-owner
cargo owner --add github:username llm-incident-manager

# List owners
cargo owner --list llm-incident-manager

# Remove owner
cargo owner --remove github:username llm-incident-manager
```

## Yanking Versions

If you need to mark a version as broken:

```bash
# Yank a version (prevents new projects from using it)
cargo yank --vers 1.0.0 llm-incident-manager

# Un-yank
cargo yank --vers 1.0.0 --undo llm-incident-manager
```

**Note**: Yanking doesn't delete the version; existing users can still use it with explicit version pins.
