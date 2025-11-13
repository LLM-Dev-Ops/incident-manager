.PHONY: help build test clean publish publish-dry-run install install-cli check fmt lint

help:
	@echo "llm-incident-manager - Makefile targets"
	@echo ""
	@echo "Development:"
	@echo "  make build           - Build the project"
	@echo "  make test            - Run tests"
	@echo "  make check           - Check code without building"
	@echo "  make fmt             - Format code"
	@echo "  make lint            - Run clippy linter"
	@echo "  make clean           - Clean build artifacts"
	@echo ""
	@echo "Publishing:"
	@echo "  make publish-dry-run - Test publishing without uploading"
	@echo "  make publish         - Publish to crates.io (requires CARGO_REGISTRY_TOKEN)"
	@echo ""
	@echo "Installation:"
	@echo "  make install         - Install both binaries locally"
	@echo "  make install-cli     - Install only the CLI tool"

# Build targets
build:
	cargo build --release

build-dev:
	cargo build

# Test targets
test:
	cargo test --all-features

test-verbose:
	cargo test --all-features -- --nocapture

# Code quality
check:
	cargo check --all-features

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-features -- -D warnings

# Clean
clean:
	cargo clean
	rm -rf target/

# Publishing
publish-dry-run:
	cargo publish --dry-run

publish:
	@if [ -z "$$CARGO_REGISTRY_TOKEN" ]; then \
		echo "Error: CARGO_REGISTRY_TOKEN not set"; \
		echo "Get your token from https://crates.io/me"; \
		echo "Then run: export CARGO_REGISTRY_TOKEN=your-token"; \
		exit 1; \
	fi
	./scripts/publish.sh

publish-script:
	./scripts/publish.sh

# Installation
install:
	cargo install --path . --bins

install-cli:
	cargo install --path . --bin llm-im-cli

# Benchmarks
bench:
	cargo bench

# Documentation
docs:
	cargo doc --no-deps --open

docs-build:
	cargo doc --no-deps

# Release builds
release: test lint
	cargo build --release
	@echo "Release build complete: target/release/llm-incident-manager"
	@echo "CLI tool: target/release/llm-im-cli"
