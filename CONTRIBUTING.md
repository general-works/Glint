# Contributing to Glint

Thank you for your interest in contributing to Glint! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct.

## Development Process

1. Fork the repository
2. Create a new branch for your feature or bugfix
3. Make your changes
4. Add tests for your changes
5. Ensure all tests pass
6. Submit a pull request

## Pull Request Process

1. Update the README.md with details of changes if needed
2. Update the documentation if needed
3. The PR will be merged once you have the sign-off of at least one other developer

## Development Setup

1. Install Rust:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/glint.git
   cd glint
   ```

3. Build the project:
   ```bash
   cargo build
   ```

4. Run tests:
   ```bash
   cargo test
   ```

## Testing

- Write unit tests for new features
- Ensure all tests pass before submitting a PR
- Run the full test suite:
  ```bash
  cargo test --all-features
  ```

## Documentation

- Add documentation for new features
- Update existing documentation if needed
- Follow the existing documentation style

## Code Style

- Follow the Rust style guide
- Use `cargo fmt` to format your code
- Use `cargo clippy` to check for common issues

## Commit Messages

- Use clear and descriptive commit messages
- Reference issues and pull requests in commit messages
- Use the present tense ("Add feature" not "Added feature")

## License

By contributing to Glint, you agree that your contributions will be licensed under the project's MIT License. 