# Contributing to Rossby

Thank you for your interest in contributing to Rossby! This document provides guidelines and instructions for contributing to this project.

## Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/mountain/rossby.git
   cd rossby
   ```

2. **Install development dependencies**
   Rossby is written in Rust, so you'll need a working Rust installation.
   ```bash
   # Install Rust if you haven't already
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install git hooks for automatic code quality checks
   ./install-hooks.sh
   ```

3. **Build the project**
   ```bash
   cargo build
   ```

4. **Run tests**
   ```bash
   cargo test
   ```

## Development Guidelines

For detailed engineering principles and development guidelines, please refer to [AGENT.md](AGENT.md). Here's a summary of the key points:

### Code Quality & Style

- **Formatting**: All code must pass `cargo fmt --check`
- **Linting**: All code must be free of `clippy` warnings (`cargo clippy -- -D warnings`)
- **Modularity**: Keep functions small and focused on a single responsibility
- **Naming**: Use descriptive, unabbreviated names

### Testing Requirements

- **Unit Tests**: All public methods must have unit tests covering happy paths, edge cases, and error conditions
- **Integration Tests**: API endpoints must have corresponding integration tests

### Documentation

- **In-Code Documentation**: All public items must have doc comments (`///`)
- **Commit Messages**: Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification

## Pull Request Process

1. **Create a Feature Branch**: Always work on a new branch with a descriptive name:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Pre-Commit Verification**: Before committing, ensure:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   cargo doc --no-deps
   ```

3. **Submit a Pull Request**: Push your branch and create a PR on GitHub
   ```bash
   git push origin feature/your-feature-name
   ```

4. **Continuous Integration**: Ensure all CI checks pass on your PR
   - Code formatting
   - Linting
   - Tests

5. **Review Process**: Wait for code review and address any feedback

## Reporting Issues

When reporting issues, please include:

- A clear description of the problem
- Steps to reproduce
- Expected vs. actual behavior
- Environment details (OS, Rust version, etc.)
- If possible, a minimal code example demonstrating the issue

## Release Process

Releases are managed by the project maintainers. If you'd like to propose a release:

1. Update the version in `Cargo.toml`
2. Update `CHANGELOG.md` with details of changes
3. Create a PR with these changes
4. After approval and merge, maintainers will tag the release and publish to crates.io

## License

By contributing to Rossby, you agree that your contributions will be licensed under the project's dual MIT OR Apache-2.0 license.
