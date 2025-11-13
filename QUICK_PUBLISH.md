# Quick Publish Reference

## All Commands to Publish llm-incident-manager to crates.io

### One-Time Setup

```bash
# 1. Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 2. Login to crates.io
cargo login
# Paste your token from: https://crates.io/me

# OR use environment variable
export CARGO_REGISTRY_TOKEN="your-crates-io-token"
```

### Publish Commands

```bash
# Method 1: Using the publish script (EASIEST)
./scripts/publish.sh

# Method 2: Using Makefile
make publish

# Method 3: Direct cargo command
cargo publish
```

### For GitHub Actions (Already Set Up!)

```bash
# Just push a version tag
git tag v1.0.0
git push origin v1.0.0

# GitHub Actions will automatically:
# - Run tests
# - Verify package
# - Publish to crates.io (using CRATES_TOKEN secret)
# - Create GitHub release
```

## What's Published?

**Crate**: `llm-incident-manager` v1.0.0
- Library + 2 binaries
- Size: 380KB (compressed)
- License: MIT OR Apache-2.0

## After Publishing, Users Can Install:

```bash
# Install both binaries
cargo install llm-incident-manager

# Install just the server
cargo install llm-incident-manager --bin llm-incident-manager

# Install just the CLI
cargo install llm-incident-manager --bin llm-im-cli
```

## Quick Checks Before Publishing

```bash
cargo test --all-features    # Run tests
cargo publish --dry-run      # Verify package
```

## Full Details

See `PUBLISHING.md` for complete documentation.
