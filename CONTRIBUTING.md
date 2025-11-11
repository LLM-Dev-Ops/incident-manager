# Contributing to LLM Incident Manager

Thank you for your interest in contributing to LLM Incident Manager! This document provides guidelines for contributing to the project.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for all contributors.

## How to Contribute

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates. When creating a bug report, include:

- **Clear title and description**
- **Steps to reproduce** the issue
- **Expected behavior** vs **actual behavior**
- **Environment details** (OS, Rust version, deployment mode)
- **Logs or error messages** (if applicable)

### Suggesting Enhancements

Enhancement suggestions are welcome! Please include:

- **Clear use case** - Why is this enhancement needed?
- **Proposed solution** - How should it work?
- **Alternatives considered** - What other approaches did you think about?
- **Impact** - Who would benefit from this?

### Pull Requests

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** following our coding standards
3. **Add tests** for new functionality
4. **Update documentation** as needed
5. **Ensure tests pass** (`cargo test`)
6. **Run formatting** (`cargo fmt`)
7. **Run linting** (`cargo clippy`)
8. **Submit your pull request**

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Git
- Docker (for integration testing)

### Getting Started

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/llm-incident-manager.git
cd llm-incident-manager

# Create a branch
git checkout -b feature/your-feature-name

# Build the project
cargo build

# Run tests
cargo test

# Run the server
cargo run
```

## Coding Standards

### Rust Style Guide

Follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/) and [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

```bash
# Format code
cargo fmt

# Check for common mistakes
cargo clippy
```

### Code Organization

- **Models** (`src/models/`) - Data structures and types
- **API** (`src/api/`) - HTTP handlers and routes
- **Processing** (`src/processing/`) - Business logic
- **State** (`src/state/`) - Storage and caching
- **Config** (`src/config.rs`) - Configuration management
- **Error** (`src/error.rs`) - Error types

### Testing

- Write unit tests for all new functionality
- Add integration tests for API endpoints
- Aim for >80% code coverage
- Use descriptive test names

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_incident_creation() {
        // Arrange
        let incident = Incident::new(/* ... */);

        // Act
        let result = create_incident(incident).await;

        // Assert
        assert!(result.is_ok());
    }
}
```

### Documentation

- Add doc comments for public functions and types
- Update README.md for user-facing changes
- Add examples for new features

```rust
/// Process an incoming alert and create an incident
///
/// # Arguments
///
/// * `alert` - The alert to process
///
/// # Returns
///
/// An `AlertAck` containing the incident ID and status
///
/// # Examples
///
/// ```
/// let alert = Alert::new(/* ... */);
/// let ack = processor.process_alert(alert).await?;
/// ```
pub async fn process_alert(&self, alert: Alert) -> Result<AlertAck> {
    // ...
}
```

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Adding or updating tests
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `chore`: Maintenance tasks

**Examples:**
```
feat(api): add GraphQL endpoint for incident queries

- Implement GraphQL schema for incidents
- Add resolver functions
- Update documentation

Closes #123
```

```
fix(dedup): resolve fingerprint collision issue

The deduplication engine was incorrectly matching incidents
with similar but not identical fingerprints. This fixes the
hash generation algorithm to include all relevant fields.

Fixes #456
```

## Testing Strategy

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test processing::

# Run with output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Start test environment
docker-compose -f docker-compose.test.yml up -d

# Run integration tests
cargo test --test integration

# Clean up
docker-compose -f docker-compose.test.yml down
```

### Performance Tests

```bash
# Run benchmarks
cargo bench
```

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag: `git tag -a v1.x.x -m "Release v1.x.x"`
4. Push tag: `git push origin v1.x.x`
5. GitHub Actions will build and publish release

## Getting Help

- **Documentation**: [README.md](README.md)
- **GitHub Issues**: [Issues](https://github.com/llm-devops/llm-incident-manager/issues)
- **Discussions**: [Discussions](https://github.com/llm-devops/llm-incident-manager/discussions)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Recognition

Contributors will be recognized in:
- `CONTRIBUTORS.md`
- GitHub contributors page
- Release notes

Thank you for contributing to LLM Incident Manager! 🎉
